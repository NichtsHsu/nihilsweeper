use crate::base::board;

pub struct GlobalConfig {
    pub chord_mode: board::ChordMode,
    pub skin: String,
    pub cell_size: u32,
    pub board: [usize; 3], // width, height, mines
}
