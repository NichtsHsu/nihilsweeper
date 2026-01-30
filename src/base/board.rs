use super::Vec2D;
use rand::{rng, seq::SliceRandom};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CellState {
    #[default]
    Closed,
    Opening(u8),
    Flagged,
    Blasted,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CellContent {
    #[default]
    Empty,
    Number(u8),
    Mine,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ChordMode {
    #[default]
    Standard,
    LeftClick,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BoardState {
    #[default]
    NotStarted,
    InProgress {
        opened_cells: usize,
        flags: usize,
    },
    Won,
    Lost {
        opened_cells: usize,
        flags: usize,
        blasted_cell: (usize, usize),
    },
}

impl CellContent {
    fn add_one_mine(&mut self) {
        match self {
            CellContent::Number(n) => *n += 1,
            CellContent::Empty => *self = CellContent::Number(1),
            _ => (),
        }
    }
}

impl BoardState {
    pub fn is_end(&self) -> bool {
        matches!(self, BoardState::Won | BoardState::Lost { .. })
    }

    pub fn cell_opened(&mut self) {
        if let BoardState::InProgress { opened_cells, .. } = self {
            *opened_cells += 1;
        }
    }

    pub fn flag_added(&mut self) {
        if let BoardState::InProgress { flags, .. } = self {
            *flags += 1;
        }
    }

    pub fn flag_removed(&mut self) {
        if let BoardState::InProgress { flags, .. } = self {
            *flags = flags.saturating_sub(1);
        }
    }

    pub fn check_win(&mut self, size: usize, mines: usize) {
        if let BoardState::InProgress { opened_cells, .. } = self
            && *opened_cells + mines == size
        {
            *self = BoardState::Won;
        }
    }

    pub fn blast(&mut self, x: usize, y: usize) {
        match self {
            BoardState::InProgress { opened_cells, flags } => {
                *self = BoardState::Lost {
                    opened_cells: *opened_cells,
                    flags: *flags,
                    blasted_cell: (x, y),
                };
            },
            BoardState::NotStarted => {
                *self = BoardState::Lost {
                    opened_cells: 0,
                    flags: 0,
                    blasted_cell: (x, y),
                }
            },
            _ => (),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EncodeType {
    Base64,
    PttUrl,
    LlamaUrl,
}

impl EncodeType {
    pub const ALL: [EncodeType; 3] = [EncodeType::Base64, EncodeType::PttUrl, EncodeType::LlamaUrl];
}

impl std::fmt::Display for EncodeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EncodeType::Base64 => write!(f, "Base64"),
            EncodeType::PttUrl => write!(f, "PTT URL"),
            EncodeType::LlamaUrl => write!(f, "Llama URL"),
        }
    }
}

pub struct ImportPack {
    pub cell_contents: Vec2D<CellContent>,
    pub mines: usize,
    pub start_position: Option<(usize, usize)>,
}

/// Build number cells based on mine positions.
///
/// Cells should not be `CellContent::Number` before calling this function.
///
/// The argument `mines` hints which algorithm to use for better performance,
/// it is not required, and can still work correctly with value 0.
pub fn build_numbers(cell_content: &mut Vec2D<CellContent>, mines: usize) {
    // If mine density > 50%, let number cells to count mines.
    // Else let mine cells to add values of number cells.
    if mines * 2 > cell_content.len() {
        for y in 0..cell_content.dims().1 {
            for x in 0..cell_content.dims().0 {
                if cell_content[(x, y)] == CellContent::Mine {
                    continue;
                }
                let mut count = 0;
                for dy in [-1isize, 0, 1] {
                    for dx in [-1isize, 0, 1] {
                        if dx == 0 && dy == 0 {
                            continue;
                        }
                        let nx = x as isize + dx;
                        let ny = y as isize + dy;
                        if nx >= 0 && ny >= 0 && cell_content.get(nx as usize, ny as usize) == Some(&CellContent::Mine)
                        {
                            count += 1;
                        }
                    }
                }
                if count > 0 {
                    cell_content[(x, y)] = CellContent::Number(count);
                }
            }
        }
    } else {
        for y in 0..cell_content.dims().1 {
            for x in 0..cell_content.dims().0 {
                if cell_content[(x, y)] != CellContent::Mine {
                    continue;
                }
                for dy in [-1isize, 0, 1] {
                    for dx in [-1isize, 0, 1] {
                        if dx == 0 && dy == 0 {
                            continue;
                        }
                        let nx = x as isize + dx;
                        let ny = y as isize + dy;
                        if nx >= 0
                            && ny >= 0
                            && let Some(c) = cell_content.get_mut(nx as usize, ny as usize)
                        {
                            c.add_one_mine()
                        }
                    }
                }
            }
        }
    }
}

pub trait Board {
    /// Get the width of the board.
    fn width(&self) -> usize;

    /// Get the height of the board.
    fn height(&self) -> usize;

    /// Get the number of mines on the board.
    fn mines(&self) -> usize;

    /// Used for no-guessing boards to indicate the first click position.
    fn start_position(&self) -> Option<(usize, usize)>;

    /// Get the current state of the board.
    fn state(&self) -> BoardState;

    /// Set the chord mode of the board.
    ///
    /// *Note*: The caller does *NOT* guarantee that mouse events are interpreted according to the
    /// configured chord mode; the implementer must handle the chord mode internally.
    fn set_chord_mode(&mut self, mode: ChordMode);

    /// Get the current chord mode of the board.
    fn chord_mode(&self) -> ChordMode;

    /// Perform a left click on the cell at `(x, y)`.
    fn left_click(&mut self, x: usize, y: usize) -> bool;

    /// Perform a right click on the cell at `(x, y)`.
    fn right_click(&mut self, x: usize, y: usize);

    /// Perform a chord click on the cell at `(x, y)`.
    ///
    /// `is_left`: `true` if the chord click is triggered by left button release, `false` if by
    /// right button release.
    fn chord_click(&mut self, x: usize, y: usize, is_left: bool) -> bool;

    /// Get the current state of the cell at `(x, y)`.
    ///
    /// *Note*: The implementer must ensure that a `Some(CellState)` is returned when `(x, y)` is
    /// within bounds, so the caller can safely `unwrap()` the result while iterating over
    /// `(0..width(), 0..height())` without causing a panic.
    fn cell_state(&self, x: usize, y: usize) -> Option<CellState>;

    /// Get all cell states.
    fn cell_states(&self) -> &Vec2D<CellState>;

    /// Get the content of the cell at `(x, y)`.
    ///
    /// *Note*: The implementer must ensure that a `Some(CellContent)` is returned when `(x, y)` is
    /// within bounds, so the caller can safely `unwrap()` the result while iterating over
    /// `(0..width(), 0..height())` without causing a panic.
    fn cell_content(&self, x: usize, y: usize) -> Option<CellContent>;

    /// Get all cell contents.
    fn cell_contents(&self) -> &Vec2D<CellContent>;

    /// Reset the board.
    fn reset(&mut self);

    /// Reset the board while keeping the mine positions.
    fn replay(&mut self);

    /// Continue playing only when the board is in `BoardState::Lost` state.
    fn resume(&mut self);
}

#[derive(Clone, Debug)]
pub struct StandardBoard {
    cell_contents: Vec2D<CellContent>,
    mines: usize,
    chord_mode: ChordMode,
    state: BoardState,
    cell_states: Vec2D<CellState>,
}

impl StandardBoard {
    fn index(&self, x: usize, y: usize) -> Option<usize> {
        if x >= self.cell_contents.dims().0 || y >= self.cell_contents.dims().1 {
            return None;
        }
        Some(y * self.cell_contents.dims().0 + x)
    }

    fn init(&mut self, click_position: Option<(usize, usize)>) {
        if self.state != BoardState::NotStarted {
            return;
        }
        let mut rng = rng();
        self.cell_contents.data_mut()[..self.mines]
            .iter_mut()
            .for_each(|c| *c = CellContent::Mine);

        if self.mines == self.cell_contents.len() {
            self.state = BoardState::InProgress {
                opened_cells: 0,
                flags: 0,
            };
            return;
        }

        if let Some((cx, cy)) = click_position
            && let Some(click_index) = self.index(cx, cy)
        {
            let len = self.cell_contents.len();
            let mut data = self.cell_contents.data_mut();
            if cx == 0 && cy == 0 {
                // Clicked on top-left corner
                data.swap(0, len - 1);
                data[1..len].shuffle(&mut rng);
            } else {
                data[..len - 1].shuffle(&mut rng);
                data[click_index..].rotate_right(1);
                data[click_index] = CellContent::Empty;
            }
        } else {
            self.cell_contents.data_mut().shuffle(&mut rng);
        }

        build_numbers(&mut self.cell_contents, self.mines);

        self.state = BoardState::InProgress {
            opened_cells: 0,
            flags: 0,
        };
    }

    fn open(&mut self, x: usize, y: usize) {
        if self.state.is_end() {
            return;
        }
        let Some(cell_state) = self.cell_states.get_mut(x, y) else {
            return;
        };
        if *cell_state != CellState::Closed && *cell_state != CellState::Flagged {
            return;
        }
        match self.cell_contents[(x, y)] {
            CellContent::Mine => {
                // It's usually not possible to reach here when `CellState::Flagged`, but just in case
                if *cell_state == CellState::Closed {
                    *cell_state = CellState::Blasted;
                    self.state.blast(x, y);
                }
            },
            CellContent::Number(n) => {
                if *cell_state == CellState::Flagged {
                    self.state.flag_removed();
                }
                *cell_state = CellState::Opening(n);
                self.state.cell_opened();
                self.state.check_win(self.cell_contents.len(), self.mines);
            },
            CellContent::Empty => {
                if *cell_state == CellState::Flagged {
                    self.state.flag_removed();
                }
                *cell_state = CellState::Opening(0);
                self.state.cell_opened();
                self.state.check_win(self.cell_contents.len(), self.mines);
                for dy in [-1isize, 0, 1] {
                    for dx in [-1isize, 0, 1] {
                        if dx == 0 && dy == 0 {
                            continue;
                        }
                        let nx = x as isize + dx;
                        let ny = y as isize + dy;
                        if nx >= 0 && ny >= 0 {
                            self.open(nx as usize, ny as usize);
                        }
                    }
                }
            },
        }
    }

    pub fn new(mut width: usize, mut height: usize, mines: usize, chord_mode: ChordMode) -> Self {
        width = width.max(1);
        height = height.max(1);
        Self {
            cell_contents: Vec2D::new(width, height),
            mines: mines.clamp(1, width * height),
            chord_mode,
            state: BoardState::NotStarted,
            cell_states: Vec2D::new(width, height),
        }
    }

    pub fn import(pack: ImportPack, chord_mode: ChordMode) -> Option<Self> {
        let ImportPack {
            cell_contents, mines, ..
        } = pack;
        let (width, height) = cell_contents.dims();
        Some(Self {
            cell_contents,
            mines,
            chord_mode,
            state: BoardState::InProgress {
                opened_cells: 0,
                flags: 0,
            },
            cell_states: Vec2D::new(width, height),
        })
    }
}

impl Board for StandardBoard {
    fn width(&self) -> usize {
        self.cell_contents.dims().0
    }

    fn height(&self) -> usize {
        self.cell_contents.dims().1
    }

    fn mines(&self) -> usize {
        self.mines
    }

    fn start_position(&self) -> Option<(usize, usize)> {
        None
    }

    fn state(&self) -> BoardState {
        self.state
    }

    fn set_chord_mode(&mut self, mode: ChordMode) {
        self.chord_mode = mode;
    }

    fn chord_mode(&self) -> ChordMode {
        self.chord_mode
    }

    fn left_click(&mut self, x: usize, y: usize) -> bool {
        if self.state.is_end() {
            return self.state.is_end();
        }
        if self.cell_contents.get(x, y).is_none() {
            return self.state.is_end();
        };
        self.init(Some((x, y)));
        if self.chord_mode == ChordMode::LeftClick
            && let CellState::Opening(1..) = self.cell_states[(x, y)]
        {
            return self.chord_click(x, y, true);
        }
        self.open(x, y);
        self.state.is_end()
    }

    fn right_click(&mut self, x: usize, y: usize) {
        if self.state.is_end() {
            return;
        }
        if self.cell_contents.get(x, y).is_none() {
            return;
        };
        self.init(None);
        let Some(cell_state) = self.cell_states.get_mut(x, y) else {
            return;
        };
        if *cell_state == CellState::Closed {
            *cell_state = CellState::Flagged;
            self.state.flag_added();
        } else if *cell_state == CellState::Flagged {
            *cell_state = CellState::Closed;
            self.state.flag_removed();
        }
    }

    fn chord_click(&mut self, x: usize, y: usize, is_left: bool) -> bool {
        if self.state.is_end() {
            return self.state.is_end();
        }
        if self.chord_mode == ChordMode::LeftClick && !is_left {
            return self.state.is_end();
        }
        if self.cell_contents.get(x, y).is_none() {
            return self.state.is_end();
        };
        self.init(None);
        if let CellState::Opening(n @ 1..) = self.cell_states[(x, y)] {
            let mut flagged_count = 0u8;
            for dy in [-1isize, 0, 1] {
                for dx in [-1isize, 0, 1] {
                    if dx == 0 && dy == 0 {
                        continue;
                    }
                    let nx = x as isize + dx;
                    let ny = y as isize + dy;
                    if nx >= 0 && ny >= 0 && self.cell_states.get(nx as usize, ny as usize) == Some(&CellState::Flagged)
                    {
                        flagged_count += 1;
                    }
                }
            }
            if flagged_count == n {
                for dy in [-1isize, 0, 1] {
                    for dx in [-1isize, 0, 1] {
                        if dx == 0 && dy == 0 {
                            continue;
                        }
                        if self.state.is_end() {
                            break;
                        }
                        let nx = x as isize + dx;
                        let ny = y as isize + dy;
                        if nx >= 0
                            && ny >= 0
                            && let Some(cell_state) = self.cell_states.get(nx as usize, ny as usize)
                            && cell_state != &CellState::Flagged
                        {
                            self.open(nx as usize, ny as usize);
                        }
                    }
                }
            }
        }

        self.state.is_end()
    }

    fn cell_state(&self, x: usize, y: usize) -> Option<CellState> {
        self.cell_states.get(x, y).cloned()
    }

    fn cell_states(&self) -> &Vec2D<CellState> {
        &self.cell_states
    }

    fn cell_content(&self, x: usize, y: usize) -> Option<CellContent> {
        self.cell_contents.get(x, y).cloned()
    }

    fn cell_contents(&self) -> &Vec2D<CellContent> {
        &self.cell_contents
    }

    fn reset(&mut self) {
        self.state = BoardState::NotStarted;
        self.cell_states.fill(CellState::Closed);
        self.cell_contents.fill(CellContent::Empty);
    }

    fn replay(&mut self) {
        self.cell_states.fill(CellState::Closed);
        self.state = BoardState::InProgress {
            opened_cells: 0,
            flags: 0,
        };
    }

    fn resume(&mut self) {
        if let BoardState::Lost {
            opened_cells,
            flags,
            blasted_cell: (x, y),
        } = self.state
        {
            if let Some(cell_state) = self.cell_states.get_mut(x, y) {
                *cell_state = CellState::Closed;
            }
            self.state = BoardState::InProgress { opened_cells, flags };
        }
    }
}
