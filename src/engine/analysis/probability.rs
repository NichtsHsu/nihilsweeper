use super::{AnalysisEngine, BoardSafety, CellSafety};

#[derive(Debug, Clone, Default)]
pub struct ProbabilityCalculator;

impl AnalysisEngine for ProbabilityCalculator {
    fn calculate(&self, board: BoardSafety) -> super::error::Result<BoardSafety> {
        Ok(board)
    }
}
