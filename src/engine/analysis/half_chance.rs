use super::{AnalysisEngine, BoardSafety, CellSafety};

#[derive(Debug, Clone, Default)]
pub struct HalfChanceCheck;

impl AnalysisEngine for HalfChanceCheck {
    fn calculate(&self, board: BoardSafety) -> super::error::Result<BoardSafety> {
        Ok(board)
    }
}
