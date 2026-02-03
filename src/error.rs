#[derive(Debug)]
pub enum Error {
    IO(std::io::Error),
    MissingResource(String),
    SkinNotFound(String),
    FileNotFound(String),
    TomlSerialize(toml::ser::Error),
    TomlDeserialize(toml::de::Error),
    Svg(usvg::Error),
    Image(image::ImageError),
    PixmapCreationFailed,
    Iced(iced::Error),
    Solver(crate::engine::solver::error::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Error::IO(value)
    }
}

impl From<usvg::Error> for Error {
    fn from(value: usvg::Error) -> Self {
        Error::Svg(value)
    }
}

impl From<iced::Error> for Error {
    fn from(value: iced::Error) -> Self {
        Error::Iced(value)
    }
}

impl From<image::ImageError> for Error {
    fn from(value: image::ImageError) -> Self {
        Error::Image(value)
    }
}

impl From<crate::engine::solver::error::Error> for Error {
    fn from(value: crate::engine::solver::error::Error) -> Self {
        Error::Solver(value)
    }
}

impl From<toml::ser::Error> for Error {
    fn from(value: toml::ser::Error) -> Self {
        Error::TomlSerialize(value)
    }
}

impl From<toml::de::Error> for Error {
    fn from(value: toml::de::Error) -> Self {
        Error::TomlDeserialize(value)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IO(e) => write!(f, "IO error: {e}"),
            Self::MissingResource(resource) => write!(f, "missing resource: {resource}"),
            Self::SkinNotFound(skin) => write!(f, "invalid skin: {skin}"),
            Self::FileNotFound(file) => write!(f, "file not found: {file}"),
            Self::TomlSerialize(e) => write!(f, "TOML serialization error: {e}"),
            Self::TomlDeserialize(e) => write!(f, "TOML deserialization error: {e}"),
            Self::Svg(e) => write!(f, "SVG error: {e}"),
            Self::Image(e) => write!(f, "image error: {e}"),
            Self::PixmapCreationFailed => write!(f, "failed to create pixmap"),
            Self::Iced(e) => write!(f, "iced error: {e}"),
            Self::Solver(e) => write!(f, "solver error: {e}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::IO(e) => e.source(),
            Self::MissingResource(_) => None,
            Self::SkinNotFound(_) => None,
            Self::FileNotFound(_) => None,
            Self::TomlSerialize(e) => e.source(),
            Self::TomlDeserialize(e) => e.source(),
            Self::Svg(e) => e.source(),
            Self::Image(e) => e.source(),
            Self::PixmapCreationFailed => None,
            Self::Iced(e) => e.source(),
            Self::Solver(e) => e.source(),
        }
    }
}
