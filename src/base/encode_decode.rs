#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EncodeType {
    Ascii,
    AsciiWithNumbers,
    Base64,
    PttUrl,
    LlamaUrl,
}

impl EncodeType {
    pub const ENCODE_TYPES: [EncodeType; 5] = [
        EncodeType::Ascii,
        EncodeType::AsciiWithNumbers,
        EncodeType::Base64,
        EncodeType::PttUrl,
        EncodeType::LlamaUrl,
    ];

    pub const DECODE_TYPES: [EncodeType; 4] = [
        EncodeType::Ascii,
        EncodeType::Base64,
        EncodeType::PttUrl,
        EncodeType::LlamaUrl,
    ];
}

impl std::fmt::Display for EncodeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EncodeType::Ascii => write!(f, "ASCII"),
            EncodeType::AsciiWithNumbers => write!(f, "ASCII (with Numbers)"),
            EncodeType::Base64 => write!(f, "Base64"),
            EncodeType::PttUrl => write!(f, "PTT URL"),
            EncodeType::LlamaUrl => write!(f, "Llama URL"),
        }
    }
}

pub mod ascii {
    use crate::base::{board::*, *};
    use log::error;

    pub fn decode(ascii: &str) -> Option<ImportPack> {
        let lines: Vec<&str> = ascii.lines().collect();
        let height = lines.len();
        if height == 0 {
            error!("ASCII input has no lines");
            return None;
        }
        let width = lines[0].chars().count();
        if width == 0 {
            error!("ASCII input has no width");
            return None;
        }

        let mut cell_contents = Vec2D::new(width, height);
        let mut mines = 0;
        let mut start_position = None;

        for (y, line) in lines.iter().enumerate() {
            if line.chars().count() != width {
                error!("Inconsistent line width in ASCII input");
                return None;
            }
            for (x, c) in line.chars().enumerate() {
                match c {
                    '*' | 'x' | 'X' => {
                        cell_contents[(x, y)] = CellContent::Mine;
                        mines += 1;
                    },
                    '.' | ' ' | '1'..='8' => {
                        cell_contents[(x, y)] = CellContent::Empty;
                    },
                    '@' => {
                        cell_contents[(x, y)] = CellContent::Empty;
                        start_position = Some((x, y));
                    },
                    _ => {
                        error!("Invalid character in ASCII input: {}", c);
                        return None;
                    },
                }
            }
        }

        build_numbers(&mut cell_contents, mines);

        Some(ImportPack {
            cell_contents,
            mines,
            start_position,
        })
    }

    pub fn encode(cell_contents: &Vec2D<CellContent>, start_position: Option<(usize, usize)>) -> String {
        let (width, height) = cell_contents.dims();
        let mut ascii = String::with_capacity(width * height + height - 1);

        for y in 0..height {
            for x in 0..width {
                if let Some((sx, sy)) = start_position
                    && (x, y) == (sx, sy)
                {
                    ascii.push('@');
                    continue;
                }
                let c = match cell_contents[(x, y)] {
                    CellContent::Mine => 'X',
                    _ => '.',
                };
                ascii.push(c);
            }
            if y < height - 1 {
                ascii.push('\n');
            }
        }

        ascii
    }

    pub fn encode_with_numbers(cell_contents: &Vec2D<CellContent>, start_position: Option<(usize, usize)>) -> String {
        let (width, height) = cell_contents.dims();
        let mut ascii = String::with_capacity(width * height + height - 1);

        for y in 0..height {
            for x in 0..width {
                if let Some((sx, sy)) = start_position
                    && (x, y) == (sx, sy)
                {
                    ascii.push('@');
                    continue;
                }
                let c = match cell_contents[(x, y)] {
                    CellContent::Mine => 'X',
                    CellContent::Number(n) => char::from_digit(n as u32, 10).unwrap(),
                    CellContent::Empty => '.',
                };
                ascii.push(c);
            }
            if y < height - 1 {
                ascii.push('\n');
            }
        }

        ascii
    }
}

pub mod base64 {
    use crate::base::{board::*, *};
    use base64::{Engine as _, engine::general_purpose::STANDARD_NO_PAD as Base64};
    use log::error;

    pub fn decode(base64: &str) -> Option<ImportPack> {
        let decoded = Base64
            .decode(base64)
            .inspect_err(|e| error!("Base64 decode error: {}", e))
            .ok()?;

        if decoded.is_empty() {
            error!("Decoded data is empty");
            return None;
        }

        let mut pos = 0;
        let byte_width = (decoded[0] & 0b0000_0111) as usize + 1;
        let has_start_pos = (decoded[0] & 0b0000_1000) != 0;
        pos += 1;

        if decoded.len() < 1 + if has_start_pos { 4 } else { 2 } + 1 {
            error!("Decoded data is too short for header");
            return None;
        }

        let mut width_bytes = [0; std::mem::size_of::<usize>()];
        width_bytes[..byte_width].copy_from_slice(&decoded[pos..pos + byte_width]);
        let width = usize::from_le_bytes(width_bytes);
        pos += byte_width;

        let mut height_bytes = [0; std::mem::size_of::<usize>()];
        height_bytes[..byte_width].copy_from_slice(&decoded[pos..pos + byte_width]);
        let height = usize::from_le_bytes(height_bytes);
        pos += byte_width;

        if width == 0 || height == 0 {
            error!("Invalid board dimensions: {}x{}", width, height);
            return None;
        }

        let mut mines = 0;
        let mut cell_contents = Vec2D::new(width, height);
        let mut start_position = None;

        if has_start_pos {
            let mut sx_bytes = [0; std::mem::size_of::<usize>()];
            sx_bytes[..byte_width].copy_from_slice(&decoded[pos..pos + byte_width]);
            let sx = usize::from_le_bytes(sx_bytes);
            pos += byte_width;

            let mut sy_bytes = [0; std::mem::size_of::<usize>()];
            sy_bytes[..byte_width].copy_from_slice(&decoded[pos..pos + byte_width]);
            let sy = usize::from_le_bytes(sy_bytes);
            pos += byte_width;

            start_position = Some((sx, sy));
        }

        for (i, cell_group) in decoded[pos..]
            .iter()
            .flat_map(|&b| (0..8).map(move |i| ((b >> i) & 1) != 0))
            .enumerate()
        {
            let x = i % width;
            let y = i / width;
            if y >= height {
                break;
            }
            if cell_group {
                cell_contents[(x, y)] = CellContent::Mine;
                mines += 1;
            } else {
                cell_contents[(x, y)] = CellContent::Empty;
            }
        }

        build_numbers(&mut cell_contents, mines);

        Some(ImportPack {
            cell_contents,
            mines,
            start_position,
        })
    }

    pub fn encode(cell_contents: &Vec2D<CellContent>, start_pos: Option<(usize, usize)>) -> String {
        let width = cell_contents.dims().0;
        let height = cell_contents.dims().1;
        let byte_width = std::cmp::max(
            (usize::BITS - width.leading_zeros()).div_ceil(8),
            (usize::BITS - height.leading_zeros()).div_ceil(8),
        )
        .max(1) as usize;

        let mut to_encode =
            Vec::with_capacity(1 + byte_width * if start_pos.is_some() { 4 } else { 2 } + (width * height).div_ceil(8));

        let first_byte = ((byte_width as u8 - 1) & 0b0000_0111) | if start_pos.is_some() { 0b0000_1000 } else { 0 };
        to_encode.push(first_byte);

        to_encode.extend_from_slice(&width.to_le_bytes()[..byte_width]);
        to_encode.extend_from_slice(&height.to_le_bytes()[..byte_width]);
        if let Some((sx, sy)) = start_pos {
            to_encode.extend_from_slice(&sx.to_le_bytes()[..byte_width]);
            to_encode.extend_from_slice(&sy.to_le_bytes()[..byte_width]);
        }

        let mut byte: u8 = 0;
        for (i, cell) in cell_contents.iter().enumerate() {
            let bit = match cell {
                CellContent::Mine => 1,
                _ => 0,
            };
            byte |= bit << (i % 8);
            if i % 8 == 7 {
                to_encode.push(byte);
                byte = 0;
            }
        }
        if !(width * height).is_multiple_of(8) {
            to_encode.push(byte);
        }

        Base64.encode(&to_encode)
    }
}

mod base32hex {
    use crate::base::{board::*, *};
    use log::error;
    use phf::{Map, phf_map};

    type BitPack = (bool, bool, bool, bool, bool);

    const ENCODE: Map<BitPack, char> = phf_map! {
        (false, false, false, false, false) => '0',
        (false, false, false, false, true) => '1',
        (false, false, false, true, false) => '2',
        (false, false, false, true, true) => '3',
        (false, false, true, false, false) => '4',
        (false, false, true, false, true) => '5',
        (false, false, true, true, false) => '6',
        (false, false, true, true, true) => '7',
        (false, true, false, false, false) => '8',
        (false, true, false, false, true) => '9',
        (false, true, false, true, false) => 'a',
        (false, true, false, true, true) => 'b',
        (false, true, true, false, false) => 'c',
        (false, true, true, false, true) => 'd',
        (false, true, true, true, false) => 'e',
        (false, true, true, true, true) => 'f',
        (true, false, false, false, false) => 'g',
        (true, false, false, false, true) => 'h',
        (true, false, false, true, false) => 'i',
        (true, false, false, true, true) => 'j',
        (true, false, true, false, false) => 'k',
        (true, false, true, false, true) => 'l',
        (true, false, true, true, false) => 'm',
        (true, false, true, true, true) => 'n',
        (true, true, false, false, false) => 'o',
        (true, true, false, false, true) => 'p',
        (true, true, false, true, false) => 'q',
        (true, true, false, true, true) => 'r',
        (true, true, true, false, false) => 's',
        (true, true, true, false, true) => 't',
        (true, true, true, true ,false) => 'u',
        (true , true, true, true, true) => 'v',
    };

    const DECODE: Map<char, BitPack> = phf_map! {
        '0' => (false, false, false, false, false),
        '1' => (false, false, false, false, true),
        '2' => (false, false, false, true, false),
        '3' => (false, false, false, true, true),
        '4' => (false, false, true, false, false),
        '5' => (false, false, true, false, true),
        '6' => (false, false, true, true, false),
        '7' => (false, false, true, true, true),
        '8' => (false, true, false, false, false),
        '9' => (false, true, false, false, true),
        'a' => (false, true, false, true, false),
        'b' => (false, true, false, true, true),
        'c' => (false, true, true, false, false),
        'd' => (false, true, true, false, true),
        'e' => (false, true, true, true, false),
        'f' => (false, true, true, true, true),
        'g' => (true, false, false, false, false),
        'h' => (true, false, false, false, true),
        'i' => (true, false, false, true, false),
        'j' => (true, false, false, true ,true),
        'k' => (true ,false ,true ,false ,false),
        'l' => (true ,false ,true ,false ,true),
        'm' => (true ,false ,true ,true ,false),
        'n' => (true ,false ,true ,true ,true),
        'o' => (true ,true ,false ,false ,false),
        'p' => (true ,true ,false ,false ,true),
        'q' => (true ,true ,false ,true ,false),
        'r' => (true ,true ,false ,true ,true),
        's' => (true ,true ,true ,false ,false),
        't' => (true ,true ,true ,false ,true),
        'u' => (true ,true ,true ,true ,false),
        'v' => (true ,true ,true ,true ,true),
    };

    pub fn decode(width: usize, height: usize, mines_str: &str) -> Option<ImportPack> {
        if !mines_str.chars().all(|c| c.is_ascii_alphanumeric()) {
            error!("Mines string contains non-alphanumeric characters");
            return None;
        }

        let mut cell_contents = Vec2D::new(width, height);
        let mut mines = 0;

        for (i, c) in mines_str.chars().enumerate() {
            let Some(bits) = DECODE.get(&c.to_ascii_lowercase()) else {
                error!("Invalid character in mines string: {}", c);
                return None;
            };
            for j in 0..5 {
                let bit = match j {
                    0 => bits.0,
                    1 => bits.1,
                    2 => bits.2,
                    3 => bits.3,
                    4 => bits.4,
                    _ => unreachable!(),
                };
                let idx = i * 5 + j;
                let x = idx % width;
                let y = idx / width;
                if y >= height {
                    break;
                }
                if bit {
                    cell_contents[(x, y)] = CellContent::Mine;
                    mines += 1;
                } else {
                    cell_contents[(x, y)] = CellContent::Empty;
                }
            }
        }

        build_numbers(&mut cell_contents, mines);
        Some(ImportPack {
            cell_contents,
            mines,
            start_position: None,
        })
    }

    pub fn encode(cell_contents: &Vec2D<CellContent>) -> (String, String) {
        let width = cell_contents.dims().0;
        let height = cell_contents.dims().1;

        let board_str = if (width, height) == (9, 9) {
            "1".to_string()
        } else if (width, height) == (16, 16) {
            "2".to_string()
        } else if (width, height) == (30, 16) {
            "3".to_string()
        } else {
            let w = if width == 0 { 1 } else { width.ilog10() as usize + 1 };
            format!("{}{:0w$}", width, height)
        };

        let mut mines_str = String::new();
        let mut bit_pack = (false, false, false, false, false);
        let mut bit_count = 0;

        for cell in cell_contents.iter() {
            let bit = matches!(cell, CellContent::Mine);
            match bit_count {
                0 => bit_pack.0 = bit,
                1 => bit_pack.1 = bit,
                2 => bit_pack.2 = bit,
                3 => bit_pack.3 = bit,
                4 => bit_pack.4 = bit,
                _ => unreachable!(),
            }
            bit_count += 1;
            if bit_count == 5 {
                mines_str.push(*ENCODE.get(&bit_pack).unwrap());
                bit_pack = (false, false, false, false, false);
                bit_count = 0;
            }
        }
        if bit_count > 0 {
            mines_str.push(*ENCODE.get(&bit_pack).unwrap());
        }

        (board_str, mines_str)
    }
}

pub mod ptt_url {
    use crate::base::{board::*, *};
    use log::{error, trace};
    use std::collections::HashMap;
    use url::Url;

    pub fn decode(url: &str) -> Option<ImportPack> {
        trace!("Decoding PTT URL: {}", url);
        let url = Url::parse(url)
            .inspect_err(|e| error!("Failed to parse URL: {}", e))
            .ok()?;
        let pairs: HashMap<_, _> = url.query_pairs().collect();
        let Some(board_str) = pairs.get("b") else {
            error!("No board 'b' parameter in URL");
            return None;
        };
        let Some(mines_str) = pairs.get("m") else {
            error!("No mines 'm' parameter in URL");
            return None;
        };

        if !board_str.chars().all(|c| c.is_ascii_digit()) {
            error!("Board string contains non-digit characters: {}", board_str);
            return None;
        }

        let (width, height) = match board_str.as_ref() {
            "1" => (9, 9),
            "2" => (16, 16),
            "3" => (30, 16),
            s => {
                let mid = s.len() / 2;
                (s[..mid].parse().unwrap_or(8), s[mid..].parse().unwrap_or(8))
            },
        };

        super::base32hex::decode(width, height, mines_str)
    }

    pub fn encode(cell_contents: &Vec2D<CellContent>) -> String {
        let (board_str, mines_str) = super::base32hex::encode(cell_contents);

        format!(
            "https://pttacgfans.github.io/Minesweeper-ZiNi-Calculator/?b={}&m={}",
            board_str, mines_str
        )
    }
}

pub mod llama_url {
    use crate::base::{board::*, *};
    use log::{error, trace};
    use std::collections::HashMap;
    use url::Url;

    pub fn decode(url: &str) -> Option<ImportPack> {
        trace!("Decoding LlamaSweeper URL: {}", url);
        let url = Url::parse(url)
            .inspect_err(|e| error!("Failed to parse URL: {}", e))
            .ok()?;
        let fragment = url.fragment()?;
        let query_str = fragment.split_once('?')?.1;
        let pairs: HashMap<_, _> = url::form_urlencoded::parse(query_str.as_bytes()).collect();
        let Some(board_str) = pairs.get("b") else {
            error!("No board 'b' parameter in URL");
            return None;
        };
        let Some(mines_str) = pairs.get("m") else {
            error!("No mines 'm' parameter in URL");
            return None;
        };

        if !board_str.chars().all(|c| c.is_ascii_digit()) {
            error!("Board string contains non-digit characters: {}", board_str);
            return None;
        }

        let (width, height) = match board_str.as_ref() {
            "1" => (9, 9),
            "2" => (16, 16),
            "3" => (30, 16),
            s => {
                let mid = s.len() / 2;
                (s[..mid].parse().unwrap_or(8), s[mid..].parse().unwrap_or(8))
            },
        };

        super::base32hex::decode(width, height, mines_str)
    }

    pub fn encode(cell_contents: &Vec2D<CellContent>) -> String {
        let (board_str, mines_str) = super::base32hex::encode(cell_contents);

        format!(
            "https://llamasweeper.com/#/game/board-editor?b={}&m={}",
            board_str, mines_str
        )
    }
}
