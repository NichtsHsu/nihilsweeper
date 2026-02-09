use super::{BoardSafety, Solver};

#[derive(Debug, Clone, Default)]
pub struct GuessingLogic;

impl Solver for GuessingLogic {
    fn calculate(&self, board: BoardSafety) -> super::error::Result<BoardSafety> {
        Ok(board)
    }
}
