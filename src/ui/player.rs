use crate::{
    base::{board, encode_decode},
    config::*,
    engine::solver,
    ui::*,
};
use iced::{Function, Task};
use log::{debug, error, info, trace};
use std::{ops::Not, sync::Arc};
use tokio::sync::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextInputType {
    Width = 0,
    Height = 1,
    Mines = 2,
    CellSize = 3,
}

#[derive(Debug, Clone)]
pub enum ExportMessage {
    ButtonClicked,
    StartExport(encode_decode::EncodeType),
    ExportCompleted(String),
    TimerTick,
}

#[derive(Debug, Clone)]
pub enum ImportMessage {
    ButtonClicked,
    StartImport(encode_decode::EncodeType, String),
    ImportCompleted,
    TimerTick,
}

#[derive(Debug, Clone)]
pub enum PlayerMessage {
    Game(game::GameMessage),
    TextInputChanged(TextInputType, String),
    CellSizeSubmit,
    ChordModeToggled(bool),
    Scrolled(iced::widget::scrollable::Viewport),
    Solver(overlay::SolverOverlayMessage),
    Export(ExportMessage),
    Import(ImportMessage),
    SyncConfigToApp(GlobalConfigUpdate),
    ShowImportModal,
    ShowExportModal,
    ShowErrorModal(String),
}

#[derive(Debug, Clone)]
pub enum ExportButtonState {
    Export,
    Exporting,
    Copied { remaining_secs: u64 },
}

#[derive(Debug, Clone)]
pub enum ImportButtonState {
    Import,
    Importing,
    Completed { remaining_secs: u64 },
}

impl From<game::GameMessage> for PlayerMessage {
    fn from(message: game::GameMessage) -> Self {
        PlayerMessage::Game(message)
    }
}

impl From<overlay::SolverOverlayMessage> for PlayerMessage {
    fn from(message: overlay::SolverOverlayMessage) -> Self {
        PlayerMessage::Solver(message)
    }
}

pub struct Player {
    config: GlobalConfig,
    config_update: GlobalConfigUpdate,
    show_probabilities: bool,
    solver_admit_flags: bool,
    skin_manager: Option<skin::SkinManager>,
    skin: Option<skin::Skin>,
    theme: iced::Theme,
    game: Option<game::Game>,
    board_to_import: Arc<Mutex<Option<Box<dyn board::Board + Send>>>>,
    text_input_states: [String; 4],
    solver_overlay: overlay::SolverOverlay,
    viewport: iced::Rectangle,
    update_solver_in_progress: bool,
    update_solver_scheduled: bool,
    import_button_state: ImportButtonState,
    export_button_state: ExportButtonState,
}

impl Player {
    const TEXT_INPUT_UPPER: [usize; 4] = [1000, 1000, 1000000, 64];
    const TEXT_INPUT_LOWER: [usize; 4] = [1, 1, 1, 8];
    const TEXT_INPUT_DEFAULTS: [usize; 4] = [30, 16, 99, 24];

    pub fn new(config: GlobalConfig) -> Self {
        let board = Box::new(board::StandardBoard::new(
            config.board[0],
            config.board[1],
            config.board[2],
            config.chord_mode,
        ));
        let skin_manager = crate::utils::resource_path("skin")
            .inspect_err(|e| error!("Failed to get skin resource path: {}", e))
            .and_then(|path| {
                skin::SkinManager::new(path).inspect_err(|e| error!("Failed to initialize SkinManager: {}", e))
            })
            .ok();
        let skin = skin_manager
            .as_ref()
            .and_then(|manager| {
                manager.skins().get(&config.skin).or_else(|| {
                    error!("Skin '{}' not found.", config.skin);
                    None
                })
            })
            .and_then(|builder| {
                builder
                    .build(config.cell_size)
                    .inspect_err(|e| error!("Failed to build skin '{}': {}", config.skin, e))
                    .ok()
            });
        let theme = if skin.as_ref().map(|s| s.light).unwrap_or_default() {
            iced::Theme::Light
        } else {
            iced::Theme::Dark
        };
        let game = skin.clone().map(|skin| game::Game::new(board, config.cell_size, skin));
        let text_input_states = [
            config.board[0].to_string(),
            config.board[1].to_string(),
            config.board[2].to_string(),
            config.cell_size.to_string(),
        ];
        let mut solver_overlay = overlay::SolverOverlay::new(
            solver::default_engine(),
            game.as_ref().map(|game| game.game_area()).unwrap_or_default(),
            game.as_ref().map(|game| game.board_area()).unwrap_or_default(),
            iced::Rectangle::default(),
            config.cell_size,
        );
        solver_overlay.update(overlay::SolverOverlayMessage::SetLightSkin(
            skin.as_ref().is_some_and(|s| s.light),
        ));
        Self {
            config,
            config_update: GlobalConfigUpdate::default(),
            show_probabilities: false,
            solver_admit_flags: false,
            skin_manager,
            skin,
            theme,
            game,
            board_to_import: Arc::new(Mutex::new(None)),
            text_input_states,
            viewport: Default::default(),
            solver_overlay,
            update_solver_in_progress: false,
            update_solver_scheduled: false,
            import_button_state: ImportButtonState::Import,
            export_button_state: ExportButtonState::Export,
        }
    }

    fn new_game(&mut self, board: Box<dyn board::Board>) -> Option<iced::Task<PlayerMessage>> {
        info!(
            "Creating new game board: {}x{} with {} mines",
            board.width(),
            board.height(),
            board.mines()
        );
        self.game = self
            .skin
            .clone()
            .map(|skin| game::Game::new(board, self.config.cell_size, skin))
            .inspect(|game| {
                self.config.board = [game.board().width(), game.board().height(), game.board().mines()];
                self.config_update.board(self.config.board);
                self.text_input_states = [
                    self.config.board[0].to_string(),
                    self.config.board[1].to_string(),
                    self.config.board[2].to_string(),
                    self.config.cell_size.to_string(),
                ];
                self.solver_overlay.clear_solver();
                self.solver_overlay.update(overlay::SolverOverlayMessage::Resize {
                    cell_size: self.config.cell_size,
                    game_area: game.game_area(),
                    board_area: game.board_area(),
                });
            });
        if let Some(game) = &mut self.game {
            game.update(game::GameMessage::ViewportChanged(self.viewport));
            if let Some(task) = self.update_solver() {
                return Some(task);
            }
        }
        None
    }

    fn update_solver(&mut self) -> Option<Task<PlayerMessage>> {
        if self.show_probabilities
            && let Some(game) = &self.game
        {
            if self.update_solver_in_progress {
                debug!("Solver update already in progress, scheduling another update");
                self.update_solver_scheduled = true;
                return None;
            }
            debug!("Starting solver update");
            self.update_solver_in_progress = true;
            return Some(
                self.solver_overlay
                    .update_solver(game.board())
                    .map(PlayerMessage::Solver),
            );
        }
        None
    }

    fn boxed_import<T: board::Board + Send + 'static>(
        &self,
        import: impl Fn(board::ImportPack, board::ChordMode) -> Option<T> + 'static,
    ) -> impl Fn(board::ImportPack) -> Option<Box<dyn board::Board + Send>> + 'static {
        let chord_mode = self.config.chord_mode;
        move |pack: board::ImportPack| {
            import(pack, chord_mode).map(|board| Box::new(board) as Box<dyn board::Board + Send>)
        }
    }

    pub fn update(&mut self, message: PlayerMessage) -> Task<PlayerMessage> {
        trace!("PlayerMessage received: {:?}", message);
        let mut tasks = vec![];
        'out: {
            match message {
                PlayerMessage::Game(msg) => {
                    trace!("Handling GameMessage: {:?}", msg);
                    let is_face_clicked = matches!(msg, game::GameMessage::FaceClicked);

                    if let game::GameMessage::ViewportChanged(viewport) = msg {
                        self.viewport = viewport;
                        self.solver_overlay.set_viewport(viewport);
                    }

                    if is_face_clicked {
                        let current_board = self
                            .game
                            .as_ref()
                            .map(|game| [game.board().width(), game.board().height(), game.board().mines()]);
                        if Some(self.config.board) != current_board {
                            info!("Board configuration changed, recreating the board");
                            let task = self.new_game(Box::new(board::StandardBoard::new(
                                self.config.board[0],
                                self.config.board[1],
                                self.config.board[2],
                                self.config.chord_mode,
                            )));
                            if let Some(task) = task {
                                tasks.push(task);
                            }
                            break 'out;
                        }
                    }

                    if let Some(game) = &mut self.game {
                        let should_update_solver = game.update(msg);
                        if is_face_clicked {
                            self.config.board = [game.board().width(), game.board().height(), game.board().mines()];
                            self.config_update.board(self.config.board);
                            self.text_input_states = [
                                self.config.board[0].to_string(),
                                self.config.board[1].to_string(),
                                self.config.board[2].to_string(),
                                self.config.cell_size.to_string(),
                            ];
                        }
                        if should_update_solver && let Some(task) = self.update_solver() {
                            tasks.push(task);
                        }
                    }
                },
                PlayerMessage::TextInputChanged(input_type, value) => {
                    trace!("TextInput changed: {:?} = '{}'", input_type, value);
                    if value.is_empty() {
                        self.text_input_states[input_type as usize] = value;
                        if input_type == TextInputType::CellSize {
                            self.config.cell_size = if let Some(game) = &self.game {
                                game.cell_size()
                            } else {
                                Player::TEXT_INPUT_DEFAULTS[TextInputType::CellSize as usize] as u32
                            };
                        } else {
                            self.config.board[input_type as usize] = if let Some(game) = &self.game {
                                match input_type {
                                    TextInputType::Width => game.board().width(),
                                    TextInputType::Height => game.board().height(),
                                    TextInputType::Mines => game.board().mines(),
                                    _ => unreachable!(),
                                }
                            } else {
                                Player::TEXT_INPUT_DEFAULTS[input_type as usize]
                            };
                        }
                        break 'out;
                    }
                    if let Ok(mut num) = value.parse::<usize>() {
                        num = num.clamp(1, Player::TEXT_INPUT_UPPER[input_type as usize]);
                        self.text_input_states[input_type as usize] = num.to_string();
                        if input_type == TextInputType::CellSize {
                            self.config.cell_size = num as u32;
                        } else {
                            self.config.board[input_type as usize] = num;
                        }
                        trace!(
                            "Text input changed: {:?} = {}, updated board config to {:?}",
                            input_type, num, self.config.board
                        );
                    }
                },
                PlayerMessage::CellSizeSubmit => {
                    debug!("CellSize submit received");
                    self.config.cell_size = self
                        .config
                        .cell_size
                        .max(Player::TEXT_INPUT_LOWER[TextInputType::CellSize as usize] as u32);
                    self.config_update.cell_size(self.config.cell_size);
                    self.text_input_states[TextInputType::CellSize as usize] = self.config.cell_size.to_string();
                    trace!("Cell size submitted: {}", self.config.cell_size);
                    let skin = self
                        .skin_manager
                        .as_ref()
                        .and_then(|manager| {
                            manager.skins().get(&self.config.skin).or_else(|| {
                                error!("Skin '{}' not found.", self.config.skin);
                                None
                            })
                        })
                        .and_then(|builder| {
                            builder
                                .build(self.config.cell_size)
                                .inspect_err(|e| error!("Failed to build skin '{}': {}", self.config.skin, e))
                                .ok()
                        });
                    if let Some(skin) = skin {
                        info!("Skin applied: {}", self.config.skin);
                        self.skin = Some(skin.clone());
                    }

                    if let Some(game) = &mut self.game
                        && let Some(skin) = &self.skin
                    {
                        game.update(game::GameMessage::Resize(self.config.cell_size, Box::new(skin.clone())));
                        self.solver_overlay.update(overlay::SolverOverlayMessage::Resize {
                            cell_size: self.config.cell_size,
                            game_area: game.game_area(),
                            board_area: game.board_area(),
                        });
                    }
                },
                PlayerMessage::ChordModeToggled(enabled) => {
                    trace!("ChordMode toggled: {}", enabled);
                    self.config.chord_mode = if enabled {
                        board::ChordMode::LeftClick
                    } else {
                        board::ChordMode::Standard
                    };
                    self.config_update.chord_mode(self.config.chord_mode);
                    debug!("Chord mode toggled: {:?}", self.config.chord_mode);
                    if let Some(game) = &mut self.game {
                        game.update(game::GameMessage::ChordModeChanged(self.config.chord_mode));
                    }
                },
                PlayerMessage::Scrolled(viewport) => {
                    trace!("Scrolled event received");
                    if let Some(game) = &mut self.game {
                        let absolute_offset = viewport.absolute_offset();
                        let bounds = viewport.bounds();
                        let viewport_rect = iced::Rectangle {
                            x: absolute_offset.x,
                            y: absolute_offset.y,
                            width: bounds.width,
                            height: bounds.height,
                        };
                        trace!("Scroll event: viewport = {:?}", viewport_rect);
                        self.viewport = viewport_rect;
                        game.update(game::GameMessage::ViewportChanged(viewport_rect));
                        self.solver_overlay.set_viewport(self.viewport);
                    }
                },
                PlayerMessage::Solver(msg) => {
                    trace!("Solver message received: {:?}", msg);
                    match &msg {
                        overlay::SolverOverlayMessage::SetEnabled(true) => {
                            debug!("Solver enabled");
                            self.show_probabilities = true;
                            self.solver_overlay.update(msg);
                            if let Some(task) = self.update_solver() {
                                tasks.push(task);
                            }
                        },
                        overlay::SolverOverlayMessage::SetEnabled(false) => {
                            debug!("Solver disabled");
                            self.show_probabilities = false;
                            self.update_solver_in_progress = false;
                            self.update_solver_scheduled = false;
                            self.solver_overlay.update(msg);
                        },
                        overlay::SolverOverlayMessage::SetAdmitFlags(admit_flags) => {
                            debug!("Solver admit flags changed: {}", admit_flags);
                            self.solver_admit_flags = *admit_flags;
                            self.solver_overlay.update(msg);
                            if let Some(task) = self.update_solver() {
                                tasks.push(task);
                            }
                        },
                        overlay::SolverOverlayMessage::SolverCompleted(..) => {
                            self.solver_overlay.update(msg);
                            self.update_solver_in_progress = false;
                            if self.update_solver_scheduled {
                                debug!("Running scheduled solver update");
                                self.update_solver_scheduled = false;
                                if let Some(task) = self.update_solver() {
                                    tasks.push(task);
                                }
                            }
                        },
                        _ => self.solver_overlay.update(msg),
                    }
                },
                PlayerMessage::Import(msg) => {
                    trace!("Import message received: {:?}", msg);
                    match msg {
                        ImportMessage::ButtonClicked => {
                            trace!("Import button clicked");
                            match self.import_button_state {
                                ImportButtonState::Import => {
                                    tasks.push(Task::done(PlayerMessage::ShowImportModal));
                                },
                                ImportButtonState::Completed { .. } => {
                                    self.import_button_state = ImportButtonState::Import
                                },
                                _ => (),
                            }
                        },
                        ImportMessage::StartImport(encode_type, string) => {
                            debug!("Start import board with type {} and text {}", encode_type, string);
                            let board_to_import = Arc::clone(&self.board_to_import);
                            let decoder = match encode_type {
                                encode_decode::EncodeType::Ascii | encode_decode::EncodeType::AsciiWithNumbers => {
                                    encode_decode::ascii::decode
                                },
                                encode_decode::EncodeType::Base64 => encode_decode::base64::decode,
                                encode_decode::EncodeType::PttUrl => encode_decode::ptt_url::decode,
                                encode_decode::EncodeType::LlamaUrl => encode_decode::llama_url::decode,
                            };
                            let import = self.boxed_import(board::StandardBoard::import);
                            self.import_button_state = ImportButtonState::Importing;
                            tasks.push(Task::perform(
                                async move {
                                    let pack = decoder(&string);
                                    let board = pack.and_then(import);
                                    let mut lock = board_to_import.lock().await;
                                    *lock = board;
                                    PlayerMessage::Import(ImportMessage::ImportCompleted)
                                },
                                std::convert::identity,
                            ));
                        },
                        ImportMessage::ImportCompleted => {
                            debug!("Import completed, retrieving board");
                            let mut board = self.board_to_import.blocking_lock().take();
                            debug!("Retrieved board from import: {}", board.is_some());
                            match board {
                                Some(board) => {
                                    if let Some(task) = self.new_game(board) {
                                        tasks.push(task);
                                    }
                                    if self.game.is_some() {
                                        info!("Board imported successfully");
                                        self.import_button_state = ImportButtonState::Completed { remaining_secs: 3 };
                                    } else {
                                        error!("Failed to import board: invalid skin");
                                        self.import_button_state = ImportButtonState::Import;
                                    }
                                },
                                None => {
                                    self.import_button_state = ImportButtonState::Import;
                                    error!("Failed to import board: invalid data");
                                    tasks.push(Task::done(PlayerMessage::ShowErrorModal(
                                        "Failed to import, check the log for details".to_string(),
                                    )));
                                },
                            };
                        },
                        ImportMessage::TimerTick => {
                            trace!("Import timer tick");
                            if let ImportButtonState::Completed { remaining_secs } = self.import_button_state {
                                if remaining_secs > 1 {
                                    self.import_button_state = ImportButtonState::Completed {
                                        remaining_secs: remaining_secs - 1,
                                    };
                                } else {
                                    self.import_button_state = ImportButtonState::Import;
                                }
                            }
                        },
                    }
                },
                PlayerMessage::Export(msg) => {
                    trace!("Export message received: {:?}", msg);
                    match msg {
                        ExportMessage::ButtonClicked => {
                            trace!("Export button clicked");
                            match self.export_button_state {
                                ExportButtonState::Export => {
                                    tasks.push(Task::done(PlayerMessage::ShowExportModal));
                                },
                                ExportButtonState::Copied { .. } => {
                                    self.export_button_state = ExportButtonState::Export
                                },
                                _ => (),
                            }
                        },
                        ExportMessage::StartExport(encode_type) => {
                            if let Some(game) = &self.game {
                                debug!("Start export board with type {}", encode_type);
                                self.export_button_state = ExportButtonState::Exporting;
                                let cell_contents = game.board().cell_contents().clone();
                                let start_pos = game.board().start_position();
                                let encoder = match encode_type {
                                    encode_decode::EncodeType::Ascii => encode_decode::ascii::encode,
                                    encode_decode::EncodeType::AsciiWithNumbers => {
                                        encode_decode::ascii::encode_with_numbers
                                    },
                                    encode_decode::EncodeType::Base64 => encode_decode::base64::encode,
                                    encode_decode::EncodeType::PttUrl => {
                                        |cell_content: &_, _| encode_decode::ptt_url::encode(cell_content)
                                    },
                                    encode_decode::EncodeType::LlamaUrl => {
                                        |cell_content: &_, _| encode_decode::llama_url::encode(cell_content)
                                    },
                                };
                                tasks.push(Task::perform(
                                    async move {
                                        let encoded = encoder(&cell_contents, start_pos);
                                        ExportMessage::ExportCompleted(encoded)
                                    },
                                    PlayerMessage::Export,
                                ));
                            }
                        },
                        ExportMessage::ExportCompleted(data) => {
                            info!("Board exported successfully to clipboard");
                            trace!("Encoded data: {}", data);
                            tasks.push(iced::clipboard::write(data));
                            self.export_button_state = ExportButtonState::Copied { remaining_secs: 3 };
                        },
                        ExportMessage::TimerTick => {
                            trace!("Export timer tick");
                            if let ExportButtonState::Copied { remaining_secs } = self.export_button_state {
                                if remaining_secs > 1 {
                                    self.export_button_state = ExportButtonState::Copied {
                                        remaining_secs: remaining_secs - 1,
                                    };
                                } else {
                                    self.export_button_state = ExportButtonState::Export;
                                }
                            }
                        },
                    }
                },
                _ => {
                    trace!("Unhandled PlayerMessage variant: {:?}", message);
                },
            }
        };
        if self.config_update.is_updated() {
            let update = std::mem::take(&mut self.config_update);
            tasks.insert(0, Task::done(PlayerMessage::SyncConfigToApp(update)));
        }

        Task::batch(tasks)
    }

    pub fn view(&self) -> iced::Element<'_, PlayerMessage> {
        let enable_button = !matches!(self.export_button_state, ExportButtonState::Exporting)
            && !matches!(self.import_button_state, ImportButtonState::Importing);

        let export_button_label = match &self.export_button_state {
            ExportButtonState::Copied { .. } => "Copied!",
            _ => "Export",
        };
        let import_button_label = match &self.import_button_state {
            ImportButtonState::Completed { .. } => "Completed!",
            _ => "Import",
        };

        let board_control = iced::widget::container(
            iced::widget::column![
                iced::widget::center_x(iced::widget::text("Board Config").size(18)),
                iced::widget::row![
                    iced::widget::text("Width:").size(16).width(iced::FillPortion(1)),
                    iced::widget::TextInput::new("width", &self.text_input_states[TextInputType::Width as usize])
                        .width(iced::FillPortion(2))
                        .on_input(PlayerMessage::TextInputChanged.with(TextInputType::Width))
                ]
                .align_y(iced::alignment::Vertical::Center),
                iced::widget::row![
                    iced::widget::text("Height:").size(16).width(iced::FillPortion(1)),
                    iced::widget::TextInput::new("height", &self.text_input_states[TextInputType::Height as usize])
                        .width(iced::FillPortion(2))
                        .on_input(PlayerMessage::TextInputChanged.with(TextInputType::Height))
                ]
                .align_y(iced::alignment::Vertical::Center),
                iced::widget::row![
                    iced::widget::text("Mines:").size(16).width(iced::FillPortion(1)),
                    iced::widget::TextInput::new("mines", &self.text_input_states[TextInputType::Mines as usize])
                        .width(iced::FillPortion(2))
                        .on_input(PlayerMessage::TextInputChanged.with(TextInputType::Mines))
                ]
                .align_y(iced::alignment::Vertical::Center),
                iced::widget::center_x(
                    iced::widget::button(iced::widget::text("New Game").align_x(iced::alignment::Horizontal::Center))
                        .width(120.0)
                        .on_press_maybe(enable_button.then_some(PlayerMessage::Game(game::GameMessage::FaceClicked)))
                ),
                iced::widget::center_x(
                    iced::widget::button(iced::widget::text("Continue").align_x(iced::alignment::Horizontal::Center))
                        .width(120.0)
                        .on_press_maybe(self.game.as_ref().and_then(|game| {
                            (enable_button && matches!(game.board().state(), board::BoardState::Lost { .. }))
                                .then_some(PlayerMessage::Game(game::GameMessage::Continue))
                        }))
                ),
                iced::widget::center_x(
                    iced::widget::button(iced::widget::text("Replay").align_x(iced::alignment::Horizontal::Center))
                        .width(120.0)
                        .on_press_maybe(self.game.as_ref().and_then(|game| {
                            (enable_button && !matches!(game.board().state(), board::BoardState::NotStarted))
                                .then_some(PlayerMessage::Game(game::GameMessage::Replay))
                        }))
                ),
                iced::widget::center_x(
                    iced::widget::button(
                        iced::widget::text(export_button_label).align_x(iced::alignment::Horizontal::Center)
                    )
                    .width(120.0)
                    .on_press_maybe(self.game.as_ref().and_then(|game| {
                        (enable_button && !matches!(game.board().state(), board::BoardState::NotStarted))
                            .then_some(PlayerMessage::Export(ExportMessage::ButtonClicked))
                    }))
                ),
                iced::widget::center_x(
                    iced::widget::button(
                        iced::widget::text(import_button_label).align_x(iced::alignment::Horizontal::Center)
                    )
                    .width(120.0)
                    .on_press_maybe(self.game.as_ref().and_then(|game| {
                        enable_button.then_some(PlayerMessage::Import(ImportMessage::ButtonClicked))
                    }))
                ),
            ]
            .spacing(4)
            .padding(6),
        )
        .style(move |theme: &iced::Theme| iced::widget::container::Style {
            border: iced::Border {
                color: theme.palette().primary,
                width: 2.0,
                radius: iced::border::radius(4.0),
            },
            ..Default::default()
        });
        let cell_size = iced::widget::row![
            iced::widget::text("Cell Size:").size(16).width(iced::FillPortion(1)),
            iced::widget::TextInput::new("cell size", &self.text_input_states[TextInputType::CellSize as usize])
                .width(iced::FillPortion(1))
                .on_input(PlayerMessage::TextInputChanged.with(TextInputType::CellSize))
                .on_submit(PlayerMessage::CellSizeSubmit),
            iced::widget::button("↵")
                .on_press(PlayerMessage::CellSizeSubmit)
                .width(iced::Length::Shrink)
        ]
        .align_y(iced::alignment::Vertical::Center);
        let control_panel = iced::widget::container(
            iced::widget::column![
                iced::widget::center_x(iced::widget::text("Control Panel").size(20)),
                iced::widget::center_x(board_control),
                iced::widget::checkbox(self.config.chord_mode == board::ChordMode::LeftClick)
                    .label("Left-click chord")
                    .on_toggle(PlayerMessage::ChordModeToggled),
                iced::widget::checkbox(self.show_probabilities)
                    .label("Show Probability")
                    .on_toggle(|enabled| PlayerMessage::Solver(overlay::SolverOverlayMessage::SetEnabled(enabled))),
                iced::widget::checkbox(self.solver_admit_flags)
                    .label("Admits Flags")
                    .on_toggle_maybe(self.show_probabilities.then_some(|admit_flags| {
                        PlayerMessage::Solver(overlay::SolverOverlayMessage::SetAdmitFlags(admit_flags))
                    })),
                cell_size
            ]
            .spacing(4)
            .padding(4),
        )
        .width(iced::Length::Fixed(200.0));
        iced::widget::scrollable(if let Some(game) = &self.game {
            iced::widget::row![
                control_panel,
                iced::widget::Stack::with_capacity(2)
                    .push(game.view().map(PlayerMessage::Game))
                    .push(self.solver_overlay.view().map(PlayerMessage::Solver))
            ]
        } else {
            iced::widget::row![
                control_panel,
                iced::widget::text("Failed to initialize the game, please check logs for details.")
                    .color(iced::Color::from_rgb8(255, 0, 0))
            ]
        })
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .direction(iced::widget::scrollable::Direction::Both {
            vertical: Default::default(),
            horizontal: Default::default(),
        })
        .on_scroll(PlayerMessage::Scrolled)
        .into()
    }

    pub fn theme(&self) -> Option<iced::Theme> {
        Some(self.theme.clone())
    }

    pub fn subscriptions(&self) -> iced::Subscription<PlayerMessage> {
        let listen = iced::event::listen_with(|event, _, _| match event {
            iced::Event::Window(iced::window::Event::Opened { size, .. }) => {
                trace!("Window opened with size: {:?}", size);
                Some(PlayerMessage::Game(game::GameMessage::ViewportChanged(
                    iced::Rectangle {
                        x: 0.0,
                        y: 0.0,
                        width: size.width,
                        height: size.height,
                    },
                )))
            },
            iced::Event::Window(iced::window::Event::Resized(size)) => {
                trace!("Window resized to size: {:?}", size);
                Some(PlayerMessage::Game(game::GameMessage::ViewportChanged(
                    iced::Rectangle {
                        x: 0.0,
                        y: 0.0,
                        width: size.width,
                        height: size.height,
                    },
                )))
            },
            _ => None,
        });

        let import_timer = match &self.import_button_state {
            ImportButtonState::Completed { .. } => iced::time::every(std::time::Duration::from_secs(1))
                .map(|_| PlayerMessage::Import(ImportMessage::TimerTick)),
            _ => iced::Subscription::none(),
        };

        let export_timer = match &self.export_button_state {
            ExportButtonState::Copied { .. } => iced::time::every(std::time::Duration::from_secs(1))
                .map(|_| PlayerMessage::Export(ExportMessage::TimerTick)),
            _ => iced::Subscription::none(),
        };

        iced::Subscription::batch([listen, import_timer, export_timer])
    }
}
