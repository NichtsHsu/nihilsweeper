use rand::{rng, seq::SliceRandom};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CellState {
    #[default]
    Closed,
    Opening(u8),
    Flagged,
    Blasted,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CellContent {
    #[default]
    Empty,
    Number(u8),
    Mine,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ChordMode {
    #[default]
    Standard,
    LeftClick,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BoardState {
    #[default]
    NotStarted,
    InProgress {
        opened_cells: usize,
        flags: usize,
    },
    Won,
    Lost {
        opened_cells: usize,
        flags: usize,
    },
}

impl BoardState {
    pub fn is_end(&self) -> bool {
        matches!(self, BoardState::Won | BoardState::Lost { .. })
    }

    pub fn cell_opened(&mut self) {
        if let BoardState::InProgress { opened_cells, .. } = self {
            *opened_cells += 1;
        }
    }

    pub fn flag_added(&mut self) {
        if let BoardState::InProgress { flags, .. } = self {
            *flags += 1;
        }
    }

    pub fn flag_removed(&mut self) {
        if let BoardState::InProgress { flags, .. } = self {
            *flags = flags.saturating_sub(1);
        }
    }

    pub fn check_win(&mut self, width: usize, height: usize, mines: usize) {
        if let BoardState::InProgress { opened_cells, .. } = self
            && *opened_cells + mines == width * height
        {
            *self = BoardState::Won;
        }
    }

    pub fn blast(&mut self) {
        match self {
            BoardState::InProgress { opened_cells, flags } => {
                *self = BoardState::Lost {
                    opened_cells: *opened_cells,
                    flags: *flags,
                };
            },
            BoardState::NotStarted => {
                *self = BoardState::Lost {
                    opened_cells: 0,
                    flags: 0,
                }
            },
            _ => (),
        }
    }
}

// #[derive(Clone, Copy, Debug, PartialEq, Eq)]
// pub struct Statistics {}

pub trait Board {
    fn width(&self) -> usize;
    fn height(&self) -> usize;
    fn mines(&self) -> usize;
    fn start_position(&self) -> Option<(usize, usize)>;
    fn state(&self) -> BoardState;
    fn set_chord_mode(&mut self, mode: ChordMode);
    fn chord_mode(&self) -> ChordMode;
    fn left_click(&mut self, x: usize, y: usize) -> bool;
    fn right_click(&mut self, x: usize, y: usize);
    fn chord_click(&mut self, x: usize, y: usize, is_left: bool) -> bool;
    fn cell_state(&self, x: usize, y: usize) -> Option<CellState>;
    fn cell_content(&self, x: usize, y: usize) -> Option<CellContent>;
    fn reset(&mut self);
}

#[derive(Clone, Debug)]
pub struct StandardBoard {
    width: usize,
    height: usize,
    mines: usize,
    chord_mode: ChordMode,
    state: BoardState,
    cell_states: Vec<CellState>,
    cell_contents: Vec<CellContent>,
}

impl StandardBoard {
    fn index(&self, x: usize, y: usize) -> Option<usize> {
        if x >= self.width || y >= self.height {
            return None;
        }
        Some(y * self.width + x)
    }

    fn index_unchecked(&self, x: usize, y: usize) -> usize {
        y * self.width + x
    }

    fn init(&mut self, click_position: Option<(usize, usize)>) {
        if self.state != BoardState::NotStarted {
            return;
        }
        let mut rng = rng();
        self.cell_contents[..self.mines]
            .iter_mut()
            .for_each(|c| *c = CellContent::Mine);
        if self.mines == self.width * self.height {
            self.state = BoardState::InProgress {
                opened_cells: 0,
                flags: 0,
            };
            return;
        }
        if let Some((cx, cy)) = click_position {
            if let Some(click_index) = self.index(cx, cy) {
                self.cell_contents[..self.width * self.height - 1].shuffle(&mut rng);
                self.cell_contents[click_index..].rotate_right(1);
                self.cell_contents[click_index] = CellContent::Empty;
            }
        } else {
            self.cell_contents.shuffle(&mut rng);
        }
        for y in 0..self.height {
            for x in 0..self.width {
                let index = self.index_unchecked(x, y);
                if self.cell_contents[index] == CellContent::Mine {
                    continue;
                }
                let mut count = 0;
                for dy in [-1isize, 0, 1] {
                    for dx in [-1isize, 0, 1] {
                        if dx == 0 && dy == 0 {
                            continue;
                        }
                        let nx = x as isize + dx;
                        let ny = y as isize + dy;
                        if nx >= 0 && nx < self.width as isize && ny >= 0 && ny < self.height as isize {
                            let n_index = self.index_unchecked(nx as usize, ny as usize);
                            if self.cell_contents[n_index] == CellContent::Mine {
                                count += 1;
                            }
                        }
                    }
                }
                if count > 0 {
                    self.cell_contents[index] = CellContent::Number(count);
                }
            }
        }
        self.state = BoardState::InProgress {
            opened_cells: 0,
            flags: 0,
        };
    }

    fn open(&mut self, x: usize, y: usize) {
        if self.state.is_end() {
            return;
        }
        let Some(index) = self.index(x, y) else {
            return;
        };
        if self.cell_states[index] != CellState::Closed && self.cell_states[index] != CellState::Flagged {
            return;
        }
        match self.cell_contents[index] {
            CellContent::Mine => {
                // It's usually not possible to reach here when `CellState::Flagged`, but just in case
                if self.cell_states[index] == CellState::Closed {
                    self.cell_states[index] = CellState::Blasted;
                    self.state.blast();
                }
            },
            CellContent::Number(n) => {
                if self.cell_states[index] == CellState::Flagged {
                    self.state.flag_removed();
                }
                self.cell_states[index] = CellState::Opening(n);
                self.state.cell_opened();
                self.state.check_win(self.width, self.height, self.mines);
            },
            CellContent::Empty => {
                if self.cell_states[index] == CellState::Flagged {
                    self.state.flag_removed();
                }
                self.cell_states[index] = CellState::Opening(0);
                self.state.cell_opened();
                self.state.check_win(self.width, self.height, self.mines);
                for dy in [-1isize, 0, 1] {
                    for dx in [-1isize, 0, 1] {
                        if dx == 0 && dy == 0 {
                            continue;
                        }
                        let nx = x as isize + dx;
                        let ny = y as isize + dy;
                        if nx >= 0 && nx < self.width as isize && ny >= 0 && ny < self.height as isize {
                            self.open(nx as usize, ny as usize);
                        }
                    }
                }
            },
        }
    }

    pub fn new(mut width: usize, mut height: usize, mut mines: usize, chord_mode: ChordMode) -> Self {
        width = width.max(1);
        height = height.max(1);
        mines = mines.clamp(1, width * height);
        let cell_states = vec![CellState::Closed; width * height];
        let cell_contents = vec![CellContent::Empty; width * height];
        Self {
            width,
            height,
            mines,
            chord_mode,
            state: BoardState::NotStarted,
            cell_states,
            cell_contents,
        }
    }
}

impl Board for StandardBoard {
    fn width(&self) -> usize {
        self.width
    }

    fn height(&self) -> usize {
        self.height
    }

    fn mines(&self) -> usize {
        self.mines
    }

    fn start_position(&self) -> Option<(usize, usize)> {
        None
    }

    fn state(&self) -> BoardState {
        self.state
    }

    fn set_chord_mode(&mut self, mode: ChordMode) {
        self.chord_mode = mode;
    }

    fn chord_mode(&self) -> ChordMode {
        self.chord_mode
    }

    fn left_click(&mut self, x: usize, y: usize) -> bool {
        if self.state.is_end() {
            return self.state.is_end();
        }
        self.init(Some((x, y)));
        if self.chord_mode == ChordMode::LeftClick
            && let CellState::Opening(1..) = self.cell_states[self.index_unchecked(x, y)]
        {
            return self.chord_click(x, y, true);
        }
        self.open(x, y);
        self.state.is_end()
    }

    fn right_click(&mut self, x: usize, y: usize) {
        if self.state.is_end() {
            return;
        }
        let Some(click_index) = self.index(x, y) else {
            return;
        };
        self.init(None);
        if self.cell_states[click_index] == CellState::Closed {
            self.cell_states[click_index] = CellState::Flagged;
            self.state.flag_added();
        } else if self.cell_states[click_index] == CellState::Flagged {
            self.cell_states[click_index] = CellState::Closed;
            self.state.flag_removed();
        }
    }

    fn chord_click(&mut self, x: usize, y: usize, is_left: bool) -> bool {
        if self.state.is_end() {
            return self.state.is_end();
        }
        if self.chord_mode == ChordMode::LeftClick && !is_left {
            return self.state.is_end();
        }
        let Some(click_index) = self.index(x, y) else {
            return self.state.is_end();
        };
        self.init(None);
        if let CellState::Opening(n @ 1..) = self.cell_states[click_index] {
            let mut flagged_count = 0u8;
            for dy in [-1isize, 0, 1] {
                for dx in [-1isize, 0, 1] {
                    if dx == 0 && dy == 0 {
                        continue;
                    }
                    let nx = x as isize + dx;
                    let ny = y as isize + dy;
                    if nx >= 0 && nx < self.width as isize && ny >= 0 && ny < self.height as isize {
                        let n_index = self.index_unchecked(nx as usize, ny as usize);
                        if self.cell_states[n_index] == CellState::Flagged {
                            flagged_count += 1;
                        }
                    }
                }
            }
            if flagged_count == n {
                for dy in [-1isize, 0, 1] {
                    for dx in [-1isize, 0, 1] {
                        if dx == 0 && dy == 0 {
                            continue;
                        }
                        if self.state.is_end() {
                            break;
                        }
                        let nx = x as isize + dx;
                        let ny = y as isize + dy;
                        if nx >= 0
                            && nx < self.width as isize
                            && ny >= 0
                            && ny < self.height as isize
                            && self.cell_states[self.index_unchecked(nx as usize, ny as usize)] != CellState::Flagged
                        {
                            self.open(nx as usize, ny as usize);
                        }
                    }
                }
            }
        }

        self.state.is_end()
    }

    fn cell_state(&self, x: usize, y: usize) -> Option<CellState> {
        self.index(x, y).map(|i| self.cell_states[i])
    }

    fn cell_content(&self, x: usize, y: usize) -> Option<CellContent> {
        self.index(x, y).map(|i| self.cell_contents[i])
    }

    fn reset(&mut self) {
        self.state = BoardState::NotStarted;
        self.cell_states.fill(CellState::Closed);
        self.cell_contents.fill(CellContent::Empty);
    }
}
