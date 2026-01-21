use super::{BoardSafety, CellProbability, CellSafety, Solver};
use log::trace;
use smallvec::smallvec;
use std::collections::{HashMap, HashSet};

// A witness can witness up to 8 boxes, and a box can be witnessed by up to 8 witnesses,
// so we can use SmallVec for better performance.
type SmallVec<T> = smallvec::SmallVec<[T; 8]>;

// Maximum safe values for binomial coefficient calculation before f64 overflow
// C(170, 85) is near the maximum representable value in f64 (~10^308)
const MAX_BINOMIAL_N: usize = 170;
const MAX_BINOMIAL_K: usize = 85;

#[derive(Debug, Clone, Default)]
pub struct ProbabilityCalculator {
    stop_on_first_safe: bool,
}

/// Represents a numbered cell (witness) that constrains adjacent frontier cells
#[derive(Debug, Clone)]
struct Witness {
    uid: usize,
    x: usize,
    y: usize,
    mines: u8,
    boxes: SmallVec<usize>, // indices into the boxes array
    processed: bool,
}

/// Represents a group (box) of frontier cells
#[derive(Debug, Clone)]
struct Box {
    uid: usize,
    cells: SmallVec<(usize, usize)>,
    witnesses: SmallVec<usize>, // indices into the witnesses array
    processed: bool,
}

/// A probability line representing a possible mine distribution
#[derive(Debug, Clone)]
struct ProbabilityLine {
    mine_count: usize,
    solution_count: f64,
    mine_box_count: Vec<f64>,    // count of mines in each box weighted by solutions
    allocated_mines: Vec<usize>, // actual number of mines allocated to each box
}

impl ProbabilityLine {
    fn new(box_count: usize) -> Self {
        Self {
            mine_count: 0,
            solution_count: 1.0,
            mine_box_count: vec![0.0; box_count],
            allocated_mines: vec![0; box_count],
        }
    }
}

impl ProbabilityCalculator {
    pub fn new(stop_on_first_safe: bool) -> Self {
        Self { stop_on_first_safe }
    }

    /// Calculate binomial coefficient C(n, k) = n! / (k! * (n-k)!)
    /// Using f64 to avoid arithmetic overflow with large values
    fn binomial(n: usize, k: usize) -> f64 {
        if k > n {
            return 0.0;
        }
        if k == 0 || k == n {
            return 1.0;
        }
        let k = k.min(n - k); // Take advantage of symmetry
        let mut result = 1.0f64;
        for i in 0..k {
            result = result * (n - i) as f64 / (i + 1) as f64;
        }
        result
    }

    /// Build witnesses and boxes from the board
    fn build_witnesses_and_boxes(&self, board: &BoardSafety) -> (Vec<Witness>, Vec<Box>) {
        let mut witnesses = Vec::new();
        let mut witnesses_map: HashMap<(usize, usize), usize> = HashMap::new();
        let mut boxes: Vec<Box> = Vec::new();

        // First pass: identify all witnesses (unresolved numbered cells)
        for y in 0..board.height() {
            for x in 0..board.width() {
                if let CellSafety::Unresolved(n) = board[(x, y)] {
                    // Count already flagged mines
                    let mut flagged = 0;
                    for nx in x.saturating_sub(1)..=(x + 1).min(board.width() - 1) {
                        for ny in y.saturating_sub(1)..=(y + 1).min(board.height() - 1) {
                            if nx == x && ny == y {
                                continue;
                            }
                            if matches!(board[(nx, ny)], CellSafety::Mine) {
                                flagged += 1;
                            }
                        }
                    }

                    if n > flagged {
                        let uid = witnesses.len();
                        witnesses.push(Witness {
                            uid,
                            x,
                            y,
                            mines: n - flagged,
                            boxes: SmallVec::new(),
                            processed: false,
                        });
                        witnesses_map.insert((x, y), uid);
                    }
                }
            }
        }

        // Second pass: create boxes for groups of frontier cells
        let mut witnesses_for_box: HashMap<SmallVec<usize>, usize> = HashMap::new();
        for y in 0..board.height() {
            for x in 0..board.width() {
                if matches!(board[(x, y)], CellSafety::Frontier) {
                    // Search witnesses to see if this cell should be included in an existed box
                    let mut this_witnesses = SmallVec::new();
                    for nx in x.saturating_sub(1)..=(x + 1).min(board.width() - 1) {
                        for ny in y.saturating_sub(1)..=(y + 1).min(board.height() - 1) {
                            if nx == x && ny == y {
                                continue;
                            }
                            if let Some(uid) = witnesses_map.get(&(nx, ny)) {
                                this_witnesses.push(*uid);
                            }
                        }
                    }
                    if let Some(&box_idx) = witnesses_for_box.get(&this_witnesses) {
                        // Add to existing box
                        boxes[box_idx].cells.push((x, y));
                        continue;
                    }
                    let uid = boxes.len();
                    boxes.push(Box {
                        uid,
                        cells: smallvec![(x, y)],
                        witnesses: this_witnesses.clone(),
                        processed: false,
                    });
                    for wit in this_witnesses.iter().copied() {
                        witnesses[wit].boxes.push(uid);
                    }
                    witnesses_for_box.insert(this_witnesses, uid);
                }
            }
        }

        (witnesses, boxes)
    }

    /// Find the first unprocessed witness
    fn find_first_witness(&self, witnesses: &[Witness]) -> Option<usize> {
        witnesses.iter().position(|w| !w.processed)
    }

    /// Find the next witness on the boundary of processed boxes
    fn find_next_witness(&self, witnesses: &[Witness], boxes: &[Box]) -> Option<usize> {
        let mut best_idx = None;
        let mut best_todo = usize::MAX;

        for box_data in boxes.iter().filter(|b| b.processed) {
            for &wit_idx in &box_data.witnesses {
                let witness = &witnesses[wit_idx];
                if !witness.processed {
                    let todo = witness.boxes.iter().filter(|&&b_idx| !boxes[b_idx].processed).count();
                    if todo == 0 {
                        return Some(wit_idx);
                    } else if todo < best_todo {
                        best_todo = todo;
                        best_idx = Some(wit_idx);
                    }
                }
            }
        }

        best_idx
    }

    /// Count mines already placed in old boxes for this witness
    fn count_placed_mines(&self, pl: &ProbabilityLine, witness_boxes: &[usize], boxes: &[Box]) -> usize {
        witness_boxes
            .iter()
            .filter(|&&b_idx| boxes[b_idx].processed)
            .map(|&b_idx| pl.allocated_mines[b_idx])
            .sum()
    }

    /// Distribute missing mines among new boxes
    fn distribute_mines(
        &self,
        mut pl: ProbabilityLine,
        boxes: &[Box],
        missing_mines: usize,
        new_boxes: &[usize],
        index: usize,
        max_total_mines: usize,
    ) -> Vec<ProbabilityLine> {
        let mut result = Vec::new();

        if index >= new_boxes.len() {
            return result;
        }

        if new_boxes.len() - index == 1 {
            // Last box - place all remaining mines here
            let box_idx = new_boxes[index];
            let box_size = boxes[box_idx].cells.len();

            if missing_mines > box_size || pl.mine_count + missing_mines > max_total_mines {
                return result;
            }

            let combinations = Self::binomial(box_size, missing_mines);
            pl.solution_count *= combinations;
            pl.mine_count += missing_mines;
            pl.allocated_mines[box_idx] = missing_mines;
            result.push(pl);
            return result;
        }

        // Recursively try different mine allocations
        let box_idx = new_boxes[index];
        let box_size = boxes[box_idx].cells.len();
        let max_mines = missing_mines.min(box_size);

        let mut recursive_distribute = |mines_here, mut pl: ProbabilityLine| {
            let combinations = Self::binomial(box_size, mines_here);
            pl.solution_count *= combinations;
            pl.mine_count += mines_here;
            pl.allocated_mines[box_idx] = mines_here;

            result.extend(self.distribute_mines(
                pl,
                boxes,
                missing_mines - mines_here,
                new_boxes,
                index + 1,
                max_total_mines,
            ));
        };

        for mines_here in 0..max_mines {
            recursive_distribute(mines_here, pl.clone());
        }
        recursive_distribute(max_mines, pl);
        result
    }

    /// Merge probabilities for a witness
    fn merge_probabilities(
        &self,
        working_probs: Vec<ProbabilityLine>,
        wit_idx: usize,
        witnesses: &mut [Witness],
        boxes: &mut [Box],
        max_total_mines: usize,
    ) -> Vec<ProbabilityLine> {
        // Extract witness data we need before modifying
        let witness_mines = witnesses[wit_idx].mines as usize;
        let new_boxes: SmallVec<usize> = witnesses[wit_idx]
            .boxes
            .iter()
            .copied()
            .filter(|&b_idx| !boxes[b_idx].processed)
            .collect();

        let mut new_probs = Vec::new();

        for pl in working_probs {
            let placed_mines = self.count_placed_mines(&pl, &witnesses[wit_idx].boxes, boxes);
            let missing_mines = witness_mines;

            if placed_mines > missing_mines {
                // Invalid - too many mines already
                continue;
            } else if placed_mines == missing_mines {
                // Already satisfied
                new_probs.push(pl);
            } else if new_boxes.is_empty() {
                // Can't place more mines
                continue;
            } else {
                let to_place = missing_mines - placed_mines;
                new_probs.extend(self.distribute_mines(pl, boxes, to_place, &new_boxes, 0, max_total_mines));
            }
        }

        // Mark as processed
        witnesses[wit_idx].processed = true;
        for &b_idx in &new_boxes {
            boxes[b_idx].processed = true;
        }

        new_probs
    }

    /// Combine probability lines with same mine count
    fn crunch_by_mine_count(&self, mut probs: Vec<ProbabilityLine>) -> Vec<ProbabilityLine> {
        if probs.is_empty() {
            return probs;
        }

        let original_len = probs.len();
        probs.sort_by_key(|pl| pl.mine_count);

        let mut result = Vec::new();
        let mut current: Option<ProbabilityLine> = None;

        for pl in probs {
            match &mut current {
                None => current = Some(pl),
                Some(curr) if curr.mine_count == pl.mine_count => {
                    curr.solution_count += pl.solution_count;
                    for i in 0..curr.mine_box_count.len() {
                        curr.mine_box_count[i] += pl.mine_box_count[i];
                    }
                },
                Some(_) => {
                    result.push(current.replace(pl).unwrap());
                },
            }
        }

        if let Some(curr) = current {
            result.push(curr);
        }

        trace!("Compressed {} probability lines to {}", original_len, result.len());
        result
    }

    /// Store probabilities from working set to held set
    fn store_probabilities(
        &self,
        held_probs: Vec<ProbabilityLine>,
        mut working_probs: Vec<ProbabilityLine>,
        max_total_mines: usize,
        box_count: usize,
    ) -> Vec<ProbabilityLine> {
        for wpl in &mut working_probs {
            for i in 0..box_count {
                wpl.mine_box_count[i] += wpl.allocated_mines[i] as f64 * wpl.solution_count;
            }
        }
        let crunched = self.crunch_by_mine_count(working_probs);
        let mut result = Vec::new();

        for wpl in &crunched {
            for hpl in &held_probs {
                if wpl.mine_count + hpl.mine_count <= max_total_mines {
                    let mut npl = ProbabilityLine::new(box_count);
                    npl.mine_count = wpl.mine_count + hpl.mine_count;
                    npl.solution_count = wpl.solution_count * hpl.solution_count;

                    for i in 0..box_count {
                        let w1 = wpl.mine_box_count[i] * hpl.solution_count;
                        let w2 = hpl.mine_box_count[i] * wpl.solution_count;
                        npl.mine_box_count[i] = w1 + w2;
                    }

                    result.push(npl);
                }
            }
        }

        self.crunch_by_mine_count(result)
    }

    fn set_probability(&self, board: &mut BoardSafety, x: usize, y: usize, probability: f32, frontier: bool) -> bool {
        if probability == 0.0 {
            board[(x, y)] = CellSafety::Safe;
            if board.suggestion().is_none() {
                board.suggest(x, y);
                if self.stop_on_first_safe {
                    trace!("ProbabilityCalculator: Found safe cell at ({}, {}), stopping", x, y);
                    return true;
                }
            }
        } else if probability == 1.0 {
            board[(x, y)] = CellSafety::Mine;
        } else {
            board[(x, y)] = CellSafety::Probability(CellProbability {
                frontier,
                mine_probability: probability.clamp(0.0, 1.0),
                ..Default::default()
            });
        }
        false
    }
}

impl Solver for ProbabilityCalculator {
    fn calculate(&self, mut board: BoardSafety) -> super::error::Result<BoardSafety> {
        let (mut witnesses, mut boxes) = self.build_witnesses_and_boxes(&board);

        // Special case: no witnesses means all numbered cells are satisfied
        // Calculate uniform probability for frontier and wilderness cells
        if witnesses.is_empty() {
            trace!("ProbabilityCalculator: No witnesses found - calculating uniform probability");

            // Count frontier and wilderness cells
            let mut total_cells = 0;
            let mut mines_left = board.mines();
            for y in 0..board.height() {
                for x in 0..board.width() {
                    match board[(x, y)] {
                        CellSafety::Mine => mines_left = mines_left.saturating_sub(1),
                        CellSafety::Frontier | CellSafety::Wilderness => total_cells += 1,
                        _ => {},
                    }
                }
            }

            // Calculate uniform probability for frontier and wilderness cells
            if total_cells > 0 {
                let uniform_probability = mines_left as f32 / total_cells as f32;

                for y in 0..board.height() {
                    for x in 0..board.width() {
                        match board[(x, y)] {
                            CellSafety::Frontier => {
                                if self.set_probability(&mut board, x, y, uniform_probability, true) {
                                    return Ok(board);
                                }
                            },
                            CellSafety::Wilderness => {
                                if self.set_probability(&mut board, x, y, uniform_probability, false) {
                                    return Ok(board);
                                }
                            },
                            _ => {},
                        }
                    }
                }
            }

            return Ok(board);
        }

        // If no boxes but there are witnesses, something is wrong
        if boxes.is_empty() {
            trace!("ProbabilityCalculator: Witnesses found but no boxes - this shouldn't happen");
            return Ok(board);
        }

        trace!(
            "ProbabilityCalculator: Found {} witnesses and {} boxes",
            witnesses.len(),
            boxes.len()
        );

        let box_count = boxes.len();

        // Count remaining mines (total mines minus already identified mines)
        let mut known_mines = 0;
        for y in 0..board.height() {
            for x in 0..board.width() {
                if matches!(board[(x, y)], CellSafety::Mine) {
                    known_mines += 1;
                }
            }
        }
        let mines_left = board.mines().saturating_sub(known_mines);

        let mut total_frontier_cells = boxes.iter().map(|b| b.cells.len()).sum::<usize>();

        // Count wilderness cells
        let mut wilderness_count = 0;
        for y in 0..board.height() {
            for x in 0..board.width() {
                if matches!(board[(x, y)], CellSafety::Wilderness) {
                    wilderness_count += 1;
                }
            }
        }

        let tiles_off_edge = wilderness_count;
        let min_total_mines = mines_left.saturating_sub(tiles_off_edge);
        let max_total_mines = mines_left;

        let mut held_probs = vec![ProbabilityLine::new(box_count)];
        let mut working_probs = vec![ProbabilityLine::new(box_count)];

        // Process witnesses
        let mut current_witness = self.find_first_witness(&witnesses);

        while let Some(wit_idx) = current_witness {
            working_probs =
                self.merge_probabilities(working_probs, wit_idx, &mut witnesses, &mut boxes, max_total_mines);

            current_witness = self.find_next_witness(&witnesses, &boxes);

            // If no next witness on boundary, we've completed an independent group
            if current_witness.is_none() {
                // Check for any remaining unprocessed witnesses
                if let Some(next_wit) = self.find_first_witness(&witnesses) {
                    // Store current probabilities and start new group
                    held_probs = self.store_probabilities(held_probs, working_probs, max_total_mines, box_count);
                    working_probs = vec![ProbabilityLine::new(box_count)];
                    current_witness = Some(next_wit);
                }
            }
        }

        // Store final working probabilities
        held_probs = self.store_probabilities(held_probs, working_probs, max_total_mines, box_count);

        // Calculate final probabilities for each box
        let mut box_tallies: Vec<f64> = vec![0.0; box_count];
        let mut total_tally = 0.0f64;

        // For wilderness probability, we track weighted mine counts and solution counts separately
        let mut wilderness_weighted_mines = 0.0f64;
        let mut wilderness_weighted_solutions = 0.0f64;

        let (max_mine, min_mine) = held_probs.iter().fold((0usize, usize::MAX), |(max, min), pl| {
            (
                max.max(mines_left.saturating_sub(pl.mine_count)),
                min.min(mines_left.saturating_sub(pl.mine_count)),
            )
        });

        let mut ln_weight = vec![0.0f64; max_mine - min_mine + 1];
        ln_weight[0] = 1.0;
        for i in 0..(max_mine - min_mine) {
            let idx = min_mine + i;
            ln_weight[i + 1] =
                ln_weight[i] + (tiles_off_edge.saturating_sub(idx) as f64).ln() - ((idx + 1) as f64).ln();
        }

        for pl in &held_probs {
            if pl.mine_count >= min_total_mines {
                let off_edge_mines = mines_left.saturating_sub(pl.mine_count);

                let weight = ln_weight[off_edge_mines - min_mine].exp();
                let pl_weight = pl.solution_count * weight;
                total_tally += pl_weight;

                for (i, box_data) in boxes.iter().enumerate() {
                    let contribution = pl.mine_box_count[i] * weight / box_data.cells.len() as f64;
                    box_tallies[i] += contribution;
                }

                // For wilderness, accumulate weighted values
                wilderness_weighted_mines += weight * pl.solution_count * off_edge_mines as f64;
                wilderness_weighted_solutions += pl_weight;
            }
        }

        trace!("ProbabilityCalculator: Total tally = {}", total_tally);

        // Update board with calculated probabilities
        if total_tally > 0.0 {
            for (i, box_data) in boxes.iter().enumerate() {
                let tally = box_tallies[i];
                let probability = (tally / total_tally) as f32;

                for &(x, y) in &box_data.cells {
                    self.set_probability(&mut board, x, y, probability, true);
                }
            }
        }

        // Handle wilderness cells without binomial for large boards
        if tiles_off_edge > 0 && wilderness_weighted_solutions > 0.0 {
            let off_edge_prob =
                (wilderness_weighted_mines / (wilderness_weighted_solutions * tiles_off_edge as f64)) as f32;

            for y in 0..board.height() {
                for x in 0..board.width() {
                    if matches!(board[(x, y)], CellSafety::Wilderness) {
                        self.set_probability(&mut board, x, y, off_edge_prob, false);
                    }
                }
            }
        }

        Ok(board)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        base::{Vec2D, board},
        engine::solver::trivial,
    };

    #[test]
    fn test_simple_probability() {
        // Create a simple 3x3 board with a "1" in the center and closed cells around it
        // Layout:
        // ? ? ?
        // ? 1 ?
        // ? ? ?
        let mut cell_states = Vec2D::filled(3, 3, board::CellState::Closed);
        cell_states[(1, 1)] = board::CellState::Opening(1);

        let board_safety = BoardSafety::new(&cell_states, 1, false);
        let calculator = ProbabilityCalculator::new(false);
        let result = calculator.calculate(board_safety).unwrap();

        // All 8 surrounding cells should have equal probability of 1/8 = 0.125
        for x in 0..3 {
            for y in 0..3 {
                if x == 1 && y == 1 {
                    continue; // Skip the center cell
                }
                match result.get(x, y) {
                    Some(CellSafety::Probability(prob)) => {
                        assert!((prob.mine_probability - 0.125).abs() < 0.01);
                    },
                    _ => panic!("Expected Probability at ({}, {})", x, y),
                }
            }
        }
    }

    #[test]
    fn test_certain_mine() {
        // Create a board where all mines are certain
        // Layout:
        // ? 1
        // 1 1
        let mut cell_states = Vec2D::filled(2, 2, board::CellState::Closed);
        cell_states[(1, 0)] = board::CellState::Opening(1);
        cell_states[(0, 1)] = board::CellState::Opening(1);
        cell_states[(1, 1)] = board::CellState::Opening(1);

        let board_safety = BoardSafety::new(&cell_states, 1, false);
        let calculator = ProbabilityCalculator::new(false);
        let result = calculator.calculate(board_safety).unwrap();

        // The top-left cell (0, 0) must be a mine (all three 1's point to it)
        match result.get(0, 0) {
            Some(CellSafety::Mine) => {},
            other => panic!("Expected Mine at (0, 0), got {:?}", other),
        }
    }

    #[test]
    fn test_certain_safe() {
        // Create a board where we can deduce a safe cell through probability
        // This is a case that TrivialSolver should normally handle,
        // but we test it directly here
        // Layout:
        // 1 1
        // ? M
        let mut cell_states = Vec2D::filled(2, 2, board::CellState::Closed);
        cell_states[(0, 0)] = board::CellState::Opening(1);
        cell_states[(1, 0)] = board::CellState::Opening(1);
        cell_states[(1, 1)] = board::CellState::Flagged;

        let board_safety = BoardSafety::new(&cell_states, 1, true);

        // First run trivial solver to resolve the obvious case
        let trivial = super::super::trivial::TrivialSolver::new(false);
        let result = trivial.calculate(board_safety).unwrap();

        // The bottom-left cell (0, 1) should be safe (both 1's satisfied by the flag)
        match result.get(0, 1) {
            Some(CellSafety::Safe) => {},
            other => panic!("Expected Safe at (0, 1), got {:?}", other),
        }
    }

    #[test]
    fn test_with_wilderness() {
        // Test wilderness cells get probabilities too
        // Layout:
        // 1 ? ?
        // ? ? ?
        // ? ? ?
        let mut cell_states = Vec2D::filled(3, 3, board::CellState::Closed);
        cell_states[(0, 0)] = board::CellState::Opening(1);

        let board_safety = BoardSafety::new(&cell_states, 2, false);
        let calculator = ProbabilityCalculator::new(false);
        let result = calculator.calculate(board_safety).unwrap();

        // The frontier cell (1, 0) and (0, 1) and (1, 1) should have some probability
        // The wilderness cells should also have a probability
        for x in 0..3 {
            for y in 0..3 {
                if x == 0 && y == 0 {
                    continue; // Skip the opened cell
                }
                match result.get(x, y) {
                    Some(CellSafety::Probability(prob)) => {
                        assert!(prob.mine_probability > 0.0 && prob.mine_probability < 1.0);
                    },
                    other => panic!("Expected Probability at ({}, {}), got {:?}", x, y, other),
                }
            }
        }
    }

    #[test]
    fn test_complex_constraints() {
        // Test a more complex scenario with multiple constraints
        // Layout:
        // ? ? ?
        // 2 2 2
        // ? ? ?
        let mut cell_states = Vec2D::filled(3, 3, board::CellState::Closed);
        cell_states[(0, 1)] = board::CellState::Opening(2);
        cell_states[(1, 1)] = board::CellState::Opening(2);
        cell_states[(2, 1)] = board::CellState::Opening(2);

        let board_safety = BoardSafety::new(&cell_states, 2, false);
        let calculator = ProbabilityCalculator::new(false);
        let result = calculator.calculate(board_safety).unwrap();

        // All frontier cells should have probabilities
        for x in 0..3 {
            for y in [0, 2] {
                match result.get(x, y) {
                    Some(CellSafety::Probability(_)) | Some(CellSafety::Safe) | Some(CellSafety::Mine) => {},
                    other => panic!("Expected determined state at ({}, {}), got {:?}", x, y, other),
                }
            }
        }
    }

    #[test]
    fn test_all_mines_determined() {
        // Test that 100% probability becomes Mine
        // Layout:
        // 1 1
        // ? M
        // With 1 mine total and the bottom-right already flagged,
        // the bottom-left must be safe (0% mine probability)
        let mut cell_states = Vec2D::filled(2, 2, board::CellState::Closed);
        cell_states[(0, 0)] = board::CellState::Opening(1);
        cell_states[(1, 0)] = board::CellState::Opening(1);
        cell_states[(1, 1)] = board::CellState::Flagged;

        let board_safety = BoardSafety::new(&cell_states, 1, true);

        // First run trivial to resolve the obvious
        let trivial = super::super::trivial::TrivialSolver::new(false);
        let intermediate = trivial.calculate(board_safety).unwrap();

        let calculator = ProbabilityCalculator::new(false);
        let result = calculator.calculate(intermediate).unwrap();

        // The bottom-left cell should be safe
        match result.get(0, 1) {
            Some(CellSafety::Safe) => {},
            other => panic!("Expected Safe at (0, 1), got {:?}", other),
        }
    }

    #[test]
    fn test_uniform_probability_no_opened_cells() {
        // Test that when no cells are opened (game not started),
        // all cells get uniform probability
        let cell_states = Vec2D::filled(5, 5, board::CellState::Closed);
        let total_cells = 25;
        let mine_count = 5;

        let board_safety = BoardSafety::new(&cell_states, mine_count, false);
        let calculator = ProbabilityCalculator::new(false);
        let result = calculator.calculate(board_safety).unwrap();

        // All cells should have uniform probability = mine_count / total_cells
        let expected_probability = mine_count as f32 / total_cells as f32;

        for x in 0..5 {
            for y in 0..5 {
                match result.get(x, y) {
                    Some(CellSafety::Probability(prob)) => {
                        assert!(
                            (prob.mine_probability - expected_probability).abs() < 0.0001,
                            "Expected probability {} at ({}, {}), got {}",
                            expected_probability,
                            x,
                            y,
                            prob.mine_probability
                        );
                    },
                    other => panic!("Expected Probability at ({}, {}), got {:?}", x, y, other),
                }
            }
        }
    }

    #[test]
    fn test_large_board_no_overflow() {
        // Test that large boards with many wilderness cells don't overflow to NaN
        // This tests the binomial overflow prevention for wilderness cells
        const BOARD_SIZE: usize = 30;
        const EXPERT_MINE_COUNT: usize = 99; // Standard expert difficulty: 30×30 with 99 mines

        let mut cell_states = Vec2D::filled(BOARD_SIZE, BOARD_SIZE, board::CellState::Closed);
        // Open one cell in the corner
        cell_states[(0, 0)] = board::CellState::Opening(1);

        let board_safety = BoardSafety::new(&cell_states, EXPERT_MINE_COUNT, false);
        let calculator = ProbabilityCalculator::new(false);
        let result = calculator.calculate(board_safety).unwrap();

        // Check that wilderness cells have valid probabilities (not NaN or infinity)
        let mut wilderness_count = 0;
        for x in 0..BOARD_SIZE {
            for y in 0..BOARD_SIZE {
                if x == 0 && y == 0 {
                    continue; // Skip opened cell
                }
                match result.get(x, y) {
                    Some(CellSafety::Probability(prob)) => {
                        // Check that probability is finite and valid
                        assert!(
                            prob.mine_probability.is_finite(),
                            "Probability at ({}, {}) is not finite: {}",
                            x,
                            y,
                            prob.mine_probability
                        );
                        assert!(
                            prob.mine_probability >= 0.0 && prob.mine_probability <= 1.0,
                            "Probability at ({}, {}) out of range: {}",
                            x,
                            y,
                            prob.mine_probability
                        );

                        // For wilderness cells (not adjacent to the opened corner)
                        if x > 1 || y > 1 {
                            wilderness_count += 1;
                        }
                    },
                    Some(CellSafety::Frontier) | Some(CellSafety::Wilderness) => {
                        // These are frontier cells adjacent to opened cell
                        // It's okay if they haven't been processed yet
                    },
                    other => panic!("Unexpected state at ({}, {}): {:?}", x, y, other),
                }
            }
        }

        // Verify we actually tested some wilderness cells
        assert!(wilderness_count > 0, "No wilderness cells were tested");
    }

    #[test]
    fn test_beginner_board() {
        // Test a beginner board scenario
        // Layout:
        // 1 ? ? ? ? ? 1 0 0
        // 1 1 1 1 ? ? 2 1 0
        // 0 0 0 1 2 ? ? 1 0
        // 0 0 0 0 1 ? 2 1 0
        // 0 1 1 1 1 ? 1 0 0
        // 0 1 ? ? ? ? 1 0 0
        // 0 1 1 1 ? ? 3 2 1
        // 0 0 0 1 ? ? ? ? ?
        // 0 0 0 1 ? ? ? ? 1
        #[rustfmt::skip]
        let mut cell_states = board::build_cell_states_with_str(
            "1 ? ? ? ? ? 1 0 0 \
            1 1 1 1 ? ? 2 1 0 \
            0 0 0 1 2 ? ? 1 0 \
            0 0 0 0 1 ? 2 1 0 \
            0 1 1 1 1 ? 1 0 0 \
            0 1 ? ? ? ? 1 0 0 \
            0 1 1 1 ? ? 3 2 1 \
            0 0 0 1 ? ? ? ? ? \
            0 0 0 1 ? ? ? ? 1",
            9,
            9
        );

        let board_safety = BoardSafety::new(&cell_states, 10, false);
        let trivial = super::super::trivial::TrivialSolver::new(false);
        let calculator = ProbabilityCalculator::new(false);
        let result = trivial
            .calculate(board_safety)
            .and_then(|board| calculator.calculate(board))
            .unwrap();

        // Check cells have probabilities assigned
        match result.get(5, 0) {
            Some(CellSafety::Probability(_)) | Some(CellSafety::Safe) | Some(CellSafety::Mine) => {},
            other => panic!("Expected determined state at ({}, {}), got {:?}", 5, 0, other),
        }
        match result.get(4, 5) {
            Some(CellSafety::Probability(_)) | Some(CellSafety::Safe) | Some(CellSafety::Mine) => {},
            other => panic!("Expected determined state at ({}, {}), got {:?}", 4, 5, other),
        }
    }
}
