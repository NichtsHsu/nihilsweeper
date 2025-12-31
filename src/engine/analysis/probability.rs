use super::{AnalysisEngine, BoardSafety, CellSafety};

#[derive(Debug, Clone, Default)]
pub struct ProbabilityCalculator {
    stop_on_first_safe: bool,
}

/// A connected block representing a group of frontier cells and their constraints.
struct ConnectedBlock {

}

impl ProbabilityCalculator {
    pub fn new(stop_on_first_safe: bool) -> Self {
        Self { stop_on_first_safe }
    }
}

impl AnalysisEngine for ProbabilityCalculator {
    fn calculate(&self, board: BoardSafety) -> super::error::Result<BoardSafety> {
        Ok(board)
    }
}
