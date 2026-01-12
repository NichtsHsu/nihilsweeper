use super::{BoardSafety, CellSafety, Solver};

#[derive(Debug, Clone, Default)]
pub struct HalfChanceCheck;

impl Solver for HalfChanceCheck {
    fn calculate(&self, board: BoardSafety) -> super::error::Result<BoardSafety> {
        Ok(board)
    }
}
