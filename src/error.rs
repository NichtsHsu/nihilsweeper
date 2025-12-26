#[derive(Debug)]
pub enum Error {
    IO(std::io::Error),
    MissingResource(String),
    SkinNotFound(String),
    FileNotFound(String),
    Svg(usvg::Error),
    Image(image::ImageError),
    PixmapCreationFailed,
    Iced(iced::Error),
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

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IO(e) => write!(f, "IO error: {e}"),
            Self::MissingResource(resource) => write!(f, "missing resource: {resource}"),
            Self::SkinNotFound(skin) => write!(f, "invalid skin: {skin}"),
            Self::FileNotFound(file) => write!(f, "file not found: {file}"),
            Self::Svg(e) => write!(f, "SVG error: {e}"),
            Self::Image(e) => write!(f, "image error: {e}"),
            Self::PixmapCreationFailed => write!(f, "failed to create pixmap"),
            Self::Iced(e) => write!(f, "iced error: {e}"),
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
            Self::Svg(e) => e.source(),
            Self::Image(e) => e.source(),
            Self::PixmapCreationFailed => None,
            Self::Iced(e) => e.source(),
        }
    }
}
