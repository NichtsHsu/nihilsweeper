use crate::base::board;

pub struct GlobalConfig {
    pub skin: String,
    pub cell_size: u32,
    pub board: [usize; 3], // width, height, mines
    pub chord_mode: board::ChordMode,
    pub show_probabilities: bool,
    pub solver_admit_flags: bool,
}
