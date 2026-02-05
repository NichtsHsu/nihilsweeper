use iced::Task;
use log::{debug, info, trace, warn};

use crate::{
    base::*,
    config::*,
    ui::{
        player::{Player, PlayerMessage},
        *,
    },
};

use iced::futures::StreamExt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BaseWindow {
    #[default]
    Player,
}

#[derive(Debug, Clone)]
pub enum AppMessage {
    Modal(modal::ModalMessage),
    Player(player::PlayerMessage),
    CloseWindow,
    ActivateWindow,
}

pub struct App {
    config: GlobalConfig,
    base_window: BaseWindow,
    current_modal: modal::Modal,
    player: player::Player,
    export: modal::export::ExportModal,
    import: modal::import::ImportModal,
    error: modal::error::ErrorModal,
}

impl App {
    pub fn new() -> Self {
        let config = GlobalConfig::load().unwrap_or_else(|err| {
            warn!("Failed to load config, using default config.");
            GlobalConfig {
                chord_mode: board::ChordMode::LeftClick,
                skin: "WoM Light".to_string(),
                cell_size: 24,
                board: [30, 16, 99],
            }
        });
        let player = Player::new(config.clone());
        Self {
            config,
            base_window: BaseWindow::default(),
            current_modal: modal::Modal::default(),
            player,
            export: modal::export::ExportModal::new(),
            import: modal::import::ImportModal::new(),
            error: modal::error::ErrorModal::new(),
        }
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
            AppMessage::ActivateWindow => {
                info!("Activating window due to second instance launch");
                // Use the unique() method to get the main window ID
                return iced::window::gain_focus(iced::window::Id::unique());
            },
            AppMessage::Player(PlayerMessage::SyncConfigToApp(config)) => {
                debug!("Applying config update: {:?}", config);
                config.apply_to(&mut self.config);
            },
            AppMessage::Player(PlayerMessage::ShowImportModal) => {
                debug!("Showing import modal");
                self.current_modal = modal::Modal::ImportGame;
            },
            AppMessage::Player(PlayerMessage::ShowExportModal) => {
                debug!("Showing export modal");
                self.current_modal = modal::Modal::ExportGame;
            },
            AppMessage::Player(PlayerMessage::ShowErrorModal(err)) => {
                debug!("Showing error modal: {}", err);
                self.error.error_message = err;
                self.current_modal = modal::Modal::Error;
            },
            AppMessage::Player(msg) => return self.player.update(msg).map(AppMessage::Player),
            AppMessage::Modal(modal::ModalMessage::Import(modal::import::ImportMessage::Cancel))
            | AppMessage::Modal(modal::ModalMessage::Export(modal::export::ExportMessage::Cancel))
            | AppMessage::Modal(modal::ModalMessage::Error(modal::error::ErrorMessage::Acknowledge)) => {
                debug!("Closing modal: {:?}", self.current_modal);
                self.current_modal = modal::Modal::None;
            },
            AppMessage::Modal(modal::ModalMessage::Import(msg)) => {
                trace!("Handling import modal message: {:?}", msg);
                if let modal::import::ImportMessage::Confirm = msg {
                    debug!("Import modal confirmed");
                    self.current_modal = modal::Modal::None;
                    let import_type = self.import.config.import_type;
                    let text = std::mem::take(&mut self.import.config.text);
                    self.import.update(msg);
                    return self
                        .player
                        .update(PlayerMessage::Import(player::ImportMessage::StartImport(
                            import_type,
                            text.text(),
                        )))
                        .map(AppMessage::Player);
                }
                self.import.update(msg);
            },
            AppMessage::Modal(modal::ModalMessage::Export(msg)) => {
                trace!("Handling export modal message: {:?}", msg);
                if let modal::export::ExportMessage::Confirm = msg {
                    debug!("Export modal confirmed");
                    self.current_modal = modal::Modal::None;
                    let export_type = self.export.config.export_type;
                    self.export.update(msg);
                    return self
                        .player
                        .update(PlayerMessage::Export(player::ExportMessage::StartExport(export_type)))
                        .map(AppMessage::Player);
                }
                self.export.update(msg);
            },
            AppMessage::CloseWindow => {
                debug!("Saving config on exit: {:?}", self.config);
                _ = self.config.save();
                return iced::exit();
            },
        }
        Task::none()
    }

    pub fn view(&self) -> iced::Element<'_, AppMessage> {
        let base = match self.base_window {
            BaseWindow::Player => self.player.view().map(AppMessage::Player),
        };
        match self.current_modal {
            modal::Modal::ImportGame => App::modal(
                base,
                self.import
                    .view()
                    .map(modal::ModalMessage::Import)
                    .map(AppMessage::Modal),
                AppMessage::Modal(modal::ModalMessage::Import(modal::import::ImportMessage::Cancel)),
            ),
            modal::Modal::ExportGame => App::modal(
                base,
                self.export
                    .view()
                    .map(modal::ModalMessage::Export)
                    .map(AppMessage::Modal),
                AppMessage::Modal(modal::ModalMessage::Export(modal::export::ExportMessage::Cancel)),
            ),
            modal::Modal::Error => App::modal(
                base,
                self.error.view().map(modal::ModalMessage::Error).map(AppMessage::Modal),
                AppMessage::Modal(modal::ModalMessage::Error(modal::error::ErrorMessage::Acknowledge)),
            ),
            modal::Modal::None => base,
        }
    }

    pub fn theme(&self) -> Option<iced::Theme> {
        self.player.theme()
    }

    pub fn subscriptions(&self) -> iced::Subscription<AppMessage> {
        let close = iced::window::close_requests().map(|_| AppMessage::CloseWindow);
        let player = self.player.subscriptions().map(AppMessage::Player);
        let activation = crate::single_instance::activation_subscription();
        iced::Subscription::batch([close, player, activation])
    }
}
