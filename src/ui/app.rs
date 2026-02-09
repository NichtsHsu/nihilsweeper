use iced::Task;
use log::{debug, info, trace, warn};

use crate::{
    base::*,
    config::*,
    ui::{main_window::MainWindow, modal::ModalMessage, player::PlayerMessage, *},
};

#[derive(Debug, Clone)]
pub enum AppMessage {
    GetWindowId(Option<iced::window::Id>),
    Modal(ModalMessage),
    Player(PlayerMessage),
    CloseWindow(iced::window::Id),
    ActivateWindow,
}

pub struct App {
    config: GlobalConfig,
    id: Option<iced::window::Id>,
    main_window: Option<MainWindow>,
    current_modal: modal::Modal,
    export: modal::export::ExportModal,
    import: modal::import::ImportModal,
    error: modal::error::ErrorModal,
}

impl App {
    pub fn new() -> (Self, Task<AppMessage>) {
        let config = GlobalConfig::load().unwrap_or_else(|err| {
            warn!("Failed to load config: {}, using default config.", err);
            GlobalConfig {
                chord_mode: board::ChordMode::LeftClick,
                skin: "WoM Light".to_string(),
                cell_size: 24,
                board: [30, 16, 99],
            }
        });

        let mut error = modal::error::ErrorModal::new();
        let mut current_modal = modal::Modal::None;
        let main_window = MainWindow::new(config.clone())
            .inspect_err(|_| {
                error.error_message = "Failed to initialize. Please check the logs for details.".to_string();
                current_modal = modal::Modal::Error;
            })
            .ok();

        (
            Self {
                config,
                id: None,
                main_window,
                current_modal,
                export: modal::export::ExportModal::new(),
                import: modal::import::ImportModal::new(),
                error,
            },
            iced::window::latest().map(AppMessage::GetWindowId),
        )
    }

    fn modal<'a, Message>(
        base: impl Into<iced::Element<'a, Message>>,
        content: impl Into<iced::Element<'a, Message>>,
        on_blur: Message,
    ) -> iced::Element<'a, Message>
    where
        Message: Clone + 'a,
    {
        iced::widget::stack![
            base.into(),
            iced::widget::opaque(
                iced::widget::mouse_area(iced::widget::center(iced::widget::opaque(content)).style(|_theme| {
                    iced::widget::container::Style {
                        background: Some(
                            iced::Color {
                                a: 0.8,
                                ..iced::Color::BLACK
                            }
                            .into(),
                        ),
                        ..iced::widget::container::Style::default()
                    }
                }))
                .on_press(on_blur)
            )
        ]
        .into()
    }

    pub fn update(&mut self, msg: AppMessage) -> Task<AppMessage> {
        trace!("AppMessage received: {:?}", msg);
        match msg {
            AppMessage::GetWindowId(id) => {
                debug!("Window ID obtained: {:?}", id);
                self.id = id;
            },
            AppMessage::ActivateWindow => {
                info!("Activating window due to second instance launch");
                return match self.id {
                    Some(id) => iced::window::gain_focus(id),
                    // Get the oldest (main) window and focus it
                    None => iced::window::oldest().and_then(iced::window::gain_focus),
                };
            },
            AppMessage::Player(PlayerMessage::Request(request)) => {
                trace!("Handling player request: {:?}", request);
                match request {
                    player::RequestMessage::SyncConfigToApp(config) => {
                        debug!("Applying config update: {:?}", config);
                        config.apply_to(&mut self.config);
                    },
                    player::RequestMessage::RegenerateSkin { skin, cell_size } => {
                        self.config.skin = skin.clone();
                        self.config.cell_size = cell_size;
                        if let Some(main_window) = &mut self.main_window {
                            return main_window.update(AppMessage::Player(PlayerMessage::Request(
                                player::RequestMessage::RegenerateSkin { skin, cell_size },
                            )));
                        };
                    },
                    player::RequestMessage::ShowImportModal => {
                        debug!("Showing import modal");
                        self.current_modal = modal::Modal::ImportGame;
                    },
                    player::RequestMessage::ShowExportModal => {
                        debug!("Showing export modal");
                        self.current_modal = modal::Modal::ExportGame;
                    },
                    player::RequestMessage::ShowErrorModal(err) => {
                        debug!("Showing error modal: {}", err);
                        self.error.error_message = err;
                        self.current_modal = modal::Modal::Error;
                    },
                    player::RequestMessage::UpdateBoardArea(..) => {
                        if let Some(main_window) = &mut self.main_window {
                            return main_window.update(AppMessage::Player(PlayerMessage::Request(request)));
                        };
                    },
                }
            },
            AppMessage::Player(..) => {
                if let Some(main_window) = &mut self.main_window {
                    return main_window.update(msg);
                };
            },
            AppMessage::Modal(ModalMessage::Import(modal::import::ImportMessage::Cancel))
            | AppMessage::Modal(ModalMessage::Export(modal::export::ExportMessage::Cancel))
            | AppMessage::Modal(ModalMessage::Error(modal::error::ErrorMessage::Acknowledge)) => {
                debug!("Closing modal: {:?}", self.current_modal);
                self.current_modal = modal::Modal::None;
            },
            AppMessage::Modal(ModalMessage::Import(msg)) => {
                trace!("Handling import modal message: {:?}", msg);
                if let modal::import::ImportMessage::Confirm = msg {
                    debug!("Import modal confirmed");
                    self.current_modal = modal::Modal::None;
                    let import_type = self.import.config.import_type;
                    let text = std::mem::take(&mut self.import.config.text);
                    self.import.update(msg);
                    if let Some(main_window) = &mut self.main_window {
                        return main_window.update(AppMessage::Player(PlayerMessage::Import(
                            player::ImportMessage::StartImport(import_type, text.text()),
                        )));
                    }
                } else {
                    self.import.update(msg);
                }
            },
            AppMessage::Modal(ModalMessage::Export(msg)) => {
                trace!("Handling export modal message: {:?}", msg);
                if let modal::export::ExportMessage::Confirm = msg {
                    debug!("Export modal confirmed");
                    self.current_modal = modal::Modal::None;
                    let export_type = self.export.config.export_type;
                    self.export.update(msg);
                    if let Some(main_window) = &mut self.main_window {
                        return main_window.update(AppMessage::Player(PlayerMessage::Export(
                            player::ExportMessage::StartExport(export_type),
                        )));
                    }
                } else {
                    self.export.update(msg);
                }
            },
            AppMessage::CloseWindow(id) => match self.id {
                Some(main_id) if main_id != id => {
                    debug!("Ignoring close request for non-main window: {:?}", id);
                },
                _ => {
                    debug!("Saving config on exit: {:?}", self.config);
                    _ = self.config.save();
                    return iced::exit();
                },
            },
        }
        Task::none()
    }

    pub fn view(&self) -> iced::Element<'_, AppMessage> {
        let base = self
            .main_window
            .as_ref()
            .map(|w| w.view())
            .unwrap_or(iced::widget::space().height(iced::Fill).width(iced::Fill).into());

        trace!("Rendering view with current modal: {:?}", self.current_modal);
        match self.current_modal {
            modal::Modal::ImportGame => App::modal(
                base,
                self.import.view().map(ModalMessage::Import).map(AppMessage::Modal),
                AppMessage::Modal(ModalMessage::Import(modal::import::ImportMessage::Cancel)),
            ),
            modal::Modal::ExportGame => App::modal(
                base,
                self.export.view().map(ModalMessage::Export).map(AppMessage::Modal),
                AppMessage::Modal(ModalMessage::Export(modal::export::ExportMessage::Cancel)),
            ),
            modal::Modal::Error => App::modal(
                base,
                self.error.view().map(ModalMessage::Error).map(AppMessage::Modal),
                AppMessage::Modal(ModalMessage::Error(modal::error::ErrorMessage::Acknowledge)),
            ),
            modal::Modal::None => base,
        }
    }

    pub fn theme(&self) -> Option<iced::Theme> {
        self.main_window.as_ref().and_then(|w| w.theme())
    }

    pub fn subscriptions(&self) -> iced::Subscription<AppMessage> {
        let mut subscriptions = Vec::with_capacity(3);
        subscriptions.push(iced::window::close_requests().map(AppMessage::CloseWindow));
        if let Some(main_window) = &self.main_window {
            subscriptions.push(main_window.subscriptions());
        }
        subscriptions.push(crate::single_instance::activation_subscription());
        iced::Subscription::batch(subscriptions)
    }
}
