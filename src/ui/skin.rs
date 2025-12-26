use log::{debug, info, trace, warn};
use std::{
    collections::HashMap,
    fs::{read_dir, read_to_string},
    path::Path,
};

mod config {
    use serde::Deserialize;

    #[derive(Clone, Debug, Default, Deserialize)]
    pub struct Skin {
        pub name: String,
        pub background_color: u32,
        pub highlight_color: u32,
        pub shadow_color: u32,
        pub border: Border,
        pub top_area: TopArea,
        pub cell: Cell,
    }

    #[derive(Clone, Debug, Default, Deserialize)]
    pub struct Border {
        pub color: u32,
        pub width_scaling: f32,
    }

    #[derive(Clone, Debug, Default, Deserialize)]
    pub struct TopArea {
        pub background_color: u32,
        pub height_scaling: f32,
        pub counter: Counter,
        pub face: Face,
    }

    #[derive(Clone, Debug, Default, Deserialize)]
    pub struct Counter {
        pub background_color: u32,
        pub minus: String,
        pub digits: [String; 10],
        pub height_scaling: f32,
        pub content_height_scaling: f32,
        pub content_width_scaling: f32,
        pub content_gap_scaling: f32,
    }

    #[derive(Clone, Debug, Default, Deserialize)]
    pub struct Face {
        pub normal: String,
        pub pressed: String,
        pub win: String,
        pub lose: String,
        pub size_scaling: f32,
    }

    #[derive(Clone, Debug, Default, Deserialize)]
    pub struct Cell {
        pub closed: String,
        pub pressed: String,
        pub opening: String,
        pub numbers: [String; 8],
        pub mine: Mine,
    }

    #[derive(Clone, Debug, Default, Deserialize)]
    pub struct Mine {
        pub flagged: String,
        pub unflagged: String,
        pub blasted: String,
        pub wrong: String,
    }
}

pub type SkinConfig = config::Skin;

mod build {
    use iced::widget::image::Handle as ImageHandle;
    use image::ImageReader;
    use log::{debug, error, trace};
    use std::path::PathBuf;

    #[derive(Debug, Clone)]
    pub struct Skin {
        pub name: String,
        pub background_color: iced::Color,
        pub highlight_color: iced::Color,
        pub shadow_color: iced::Color,
        pub border: Border,
        pub top_area: TopArea,
        pub cell: Cell,
    }

    #[derive(Debug, Clone)]
    pub struct Border {
        pub color: iced::Color,
        pub width: f32,
    }

    #[derive(Debug, Clone)]
    pub struct TopArea {
        pub background_color: iced::Color,
        pub height: f32,
        pub counter: Counter,
        pub face: Face,
    }

    #[derive(Debug, Clone)]
    pub struct Counter {
        pub background_color: iced::Color,
        pub minus: ImageHandle,
        pub digits: [ImageHandle; 10],
        pub height: f32,
        pub content_height: f32,
        pub content_width: f32,
        pub content_gap: f32,
    }

    #[derive(Debug, Clone)]
    pub struct Face {
        pub normal: ImageHandle,
        pub pressed: ImageHandle,
        pub win: ImageHandle,
        pub lose: ImageHandle,
        pub size: f32,
    }

    #[derive(Debug, Clone)]
    pub struct Cell {
        pub closed: ImageHandle,
        pub pressed: ImageHandle,
        pub opening: ImageHandle,
        pub numbers: [ImageHandle; 8],
        pub mine: Mine,
    }

    #[derive(Debug, Clone)]
    pub struct Mine {
        pub flagged: ImageHandle,
        pub unflagged: ImageHandle,
        pub blasted: ImageHandle,
        pub wrong: ImageHandle,
    }

    #[derive(Debug, Clone)]
    pub struct SkinBuilder {
        pub dir: PathBuf,
        pub config: super::SkinConfig,
    }

    impl SkinBuilder {
        fn load_image(&self, file: &str, width: u32, height: u32) -> crate::error::Result<ImageHandle> {
            let path = self.dir.join(file);
            trace!("Loading image file: {}", path.to_string_lossy());
            if path
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| s.eq_ignore_ascii_case("svg"))
                == Some(true)
            {
                if !path.try_exists()? {
                    error!("File not found: {}", path.to_string_lossy());
                    return Err(crate::error::Error::FileNotFound(path.to_string_lossy().to_string()));
                }
                trace!("Loading file as an SVG: {}", path.to_string_lossy());
                let svg_bytes = std::fs::read(&path)
                    .inspect_err(|e| error!("Failed to read SVG file {}: {}", path.to_string_lossy(), e))?;
                let opt = usvg::Options::default();
                let tree = usvg::Tree::from_data(&svg_bytes, &opt)
                    .inspect_err(|e| error!("Failed to parse SVG file {}: {}", path.to_string_lossy(), e))?;
                let mut pixmap = tiny_skia::Pixmap::new(width, height).ok_or_else(|| {
                    error!(
                        "Failed to create pixmap for SVG file {} with size {}x{}",
                        path.to_string_lossy(),
                        width,
                        height
                    );
                    crate::error::Error::PixmapCreationFailed
                })?;

                let w = tree.size().width();
                let h = tree.size().height();
                let scale_w = width as f32 / w;
                let scale_h = height as f32 / h;

                resvg::render(
                    &tree,
                    tiny_skia::Transform::from_scale(scale_w, scale_h),
                    &mut pixmap.as_mut(),
                );

                Ok(ImageHandle::from_rgba(width, height, pixmap.take()))
            } else {
                trace!("Loading file as a raster image: {}", path.to_string_lossy());
                let img = ImageReader::open(path)
                    .inspect_err(|e| error!("Failed to open image file: {}", e))?
                    .decode()
                    .inspect_err(|e| error!("Failed to decode image file: {}", e))?
                    .resize_exact(width, height, image::imageops::Lanczos3);
                let rgba = img.to_rgba8().into_raw();
                Ok(ImageHandle::from_rgba(width, height, rgba))
            }
        }

        pub fn build(&self, cell_size: u32) -> crate::error::Result<Skin> {
            debug!("Building skin: {}", self.config.name);
            let border = Border {
                color: iced::Color::from_rgb8(
                    ((self.config.border.color >> 16) & 0xFF) as u8,
                    ((self.config.border.color >> 8) & 0xFF) as u8,
                    (self.config.border.color & 0xFF) as u8,
                ),
                width: (self.config.border.width_scaling * cell_size as f32).ceil(),
            };
            let content_height = (self.config.top_area.counter.content_height_scaling * cell_size as f32).ceil();
            let content_width = (self.config.top_area.counter.content_width_scaling * cell_size as f32).ceil();
            let counter = Counter {
                background_color: iced::Color::from_rgb8(
                    ((self.config.top_area.counter.background_color >> 16) & 0xFF) as u8,
                    ((self.config.top_area.counter.background_color >> 8) & 0xFF) as u8,
                    (self.config.top_area.counter.background_color & 0xFF) as u8,
                ),
                minus: self.load_image(
                    &self.config.top_area.counter.minus,
                    content_width as u32,
                    content_height as u32,
                )?,
                digits: self
                    .config
                    .top_area
                    .counter
                    .digits
                    .iter()
                    .map(|file| self.load_image(file, content_width as u32, content_height as u32))
                    .collect::<Result<Vec<_>, _>>()?
                    .try_into()
                    .unwrap(),
                height: (self.config.top_area.counter.height_scaling * cell_size as f32).ceil(),
                content_height,
                content_width,
                content_gap: (self.config.top_area.counter.content_gap_scaling * cell_size as f32).ceil(),
            };
            let size = (self.config.top_area.face.size_scaling * cell_size as f32).ceil();
            let face = Face {
                normal: self.load_image(&self.config.top_area.face.normal, size as u32, size as u32)?,
                pressed: self.load_image(&self.config.top_area.face.pressed, size as u32, size as u32)?,
                win: self.load_image(&self.config.top_area.face.win, size as u32, size as u32)?,
                lose: self.load_image(&self.config.top_area.face.lose, size as u32, size as u32)?,
                size,
            };
            let top_area = TopArea {
                background_color: iced::Color::from_rgb8(
                    ((self.config.top_area.background_color >> 16) & 0xFF) as u8,
                    ((self.config.top_area.background_color >> 8) & 0xFF) as u8,
                    (self.config.top_area.background_color & 0xFF) as u8,
                ),
                height: (self.config.top_area.height_scaling * cell_size as f32).ceil(),
                counter,
                face,
            };
            let mine = Mine {
                flagged: self.load_image(&self.config.cell.mine.flagged, cell_size, cell_size)?,
                unflagged: self.load_image(&self.config.cell.mine.unflagged, cell_size, cell_size)?,
                blasted: self.load_image(&self.config.cell.mine.blasted, cell_size, cell_size)?,
                wrong: self.load_image(&self.config.cell.mine.wrong, cell_size, cell_size)?,
            };
            let cell = Cell {
                closed: self.load_image(&self.config.cell.closed, cell_size, cell_size)?,
                pressed: self.load_image(&self.config.cell.pressed, cell_size, cell_size)?,
                opening: self.load_image(&self.config.cell.opening, cell_size, cell_size)?,
                numbers: self
                    .config
                    .cell
                    .numbers
                    .iter()
                    .map(|file| self.load_image(file, cell_size, cell_size))
                    .collect::<Result<Vec<_>, _>>()?
                    .try_into()
                    .unwrap(),
                mine,
            };
            debug!("Skin built successfully: {}", self.config.name);
            Ok(Skin {
                name: self.config.name.clone(),
                background_color: iced::Color::from_rgb8(
                    ((self.config.background_color >> 16) & 0xFF) as u8,
                    ((self.config.background_color >> 8) & 0xFF) as u8,
                    (self.config.background_color & 0xFF) as u8,
                ),
                highlight_color: iced::Color::from_rgb8(
                    ((self.config.highlight_color >> 16) & 0xFF) as u8,
                    ((self.config.highlight_color >> 8) & 0xFF) as u8,
                    (self.config.highlight_color & 0xFF) as u8,
                ),
                shadow_color: iced::Color::from_rgb8(
                    ((self.config.shadow_color >> 16) & 0xFF) as u8,
                    ((self.config.shadow_color >> 8) & 0xFF) as u8,
                    (self.config.shadow_color & 0xFF) as u8,
                ),
                border,
                top_area,
                cell,
            })
        }
    }
}

pub use build::{Skin, SkinBuilder};

#[derive(Debug)]
pub struct SkinManager {
    skins: HashMap<String, SkinBuilder>,
}

impl SkinManager {
    pub fn new(root: impl AsRef<Path>) -> crate::error::Result<Self> {
        let mut skins = HashMap::new();
        for entry in read_dir(root)? {
            let entry = match entry {
                Ok(a) => a,
                Err(e) => {
                    warn!("Failed to read skin directory entry: {e}, skipped");
                    continue;
                },
            };
            let file_type = match entry.file_type() {
                Ok(a) => a,
                Err(e) => {
                    warn!(
                        "Failed to get file type for {}: {e}, skipped",
                        entry.path().to_string_lossy()
                    );
                    continue;
                },
            };

            trace!("Reading entry {}", entry.path().to_string_lossy());
            if file_type.is_dir() {
                let skin_config = entry.path().join("skin.toml");

                match skin_config.try_exists() {
                    Ok(true) => debug!("Found {}", skin_config.to_string_lossy()),
                    Ok(false) => {
                        warn!("Skin config not found: {}, skipped", skin_config.to_string_lossy());
                        continue;
                    },
                    Err(e) => {
                        warn!(
                            "Failed to check existence of {}: {e}, skipped",
                            skin_config.to_string_lossy()
                        );
                        continue;
                    },
                }

                let skin_config = read_to_string(skin_config)?;
                let skin_config: SkinConfig = match toml::from_str(&skin_config) {
                    Ok(a) => a,
                    Err(e) => {
                        warn!("Failed to load skin: {e}, skipped");
                        continue;
                    },
                };

                info!("Loaded skin: {}", skin_config.name);
                trace!("{skin_config:?}");

                skins.insert(
                    skin_config.name.clone(),
                    SkinBuilder {
                        dir: entry.path(),
                        config: skin_config,
                    },
                );
            } else {
                trace!("Skipping non-directory entry: {}", entry.path().to_string_lossy());
            }
        }

        if skins.is_empty() {
            Err(crate::error::Error::MissingResource("skin".to_string()))
        } else {
            Ok(Self { skins })
        }
    }

    pub fn skins(&self) -> &HashMap<String, SkinBuilder> {
        &self.skins
    }
}
