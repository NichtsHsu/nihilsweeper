#[derive(Debug, Clone)]
pub enum Error {
    MinesNotSatisfied {
        x: usize,
        y: usize,
        expected: u8,
        actual: u8,
    },
    TooManyMines(usize),
    TooFewMines(usize),
}

pub type Result<T> = std::result::Result<T, Error>;

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::MinesNotSatisfied { x, y, expected, actual } => {
                write!(
                    f,
                    "Mines not satisfied at ({}, {}): expected {}, found {}",
                    x, y, expected, actual
                )
            },
            Error::TooManyMines(actual) => {
                write!(f, "{} mines is too many to complete the board", actual)
            },
            Error::TooFewMines(actual) => {
                write!(f, "{} mines is too few to complete the board", actual)
            },
        }
    }
}

impl std::error::Error for Error {}
