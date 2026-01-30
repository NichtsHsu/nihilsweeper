use iced::Task;
use log::warn;

use crate::ui::{
    player::{Player, PlayerMessage},
    *,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BaseWindow {
    #[default]
    Player,
}

#[derive(Debug, Clone)]
pub enum AppMessage {
    Modal(modal::ModalMessage),
    Player(player::PlayerMessage),
}

pub struct App {
    base_window: BaseWindow,
    current_modal: modal::Modal,
    player: player::Player,
    export: modal::export::ExportModal,
    import: modal::import::ImportModal,
    error: modal::error::ErrorModal,
}

impl App {
    pub fn new() -> Self {
        Self {
            base_window: BaseWindow::default(),
            current_modal: modal::Modal::default(),
            player: player::Player::new(),
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
        match msg {
            AppMessage::Player(PlayerMessage::ShowImportModal) => {
                self.current_modal = modal::Modal::ImportGame;
            },
            AppMessage::Player(PlayerMessage::ShowExportModal) => {
                self.current_modal = modal::Modal::ExportGame;
            },
            AppMessage::Player(PlayerMessage::ShowErrorModal(err)) => {
                self.error.error_message = err;
                self.current_modal = modal::Modal::Error;
            },
            AppMessage::Player(msg) => return self.player.update(msg).map(AppMessage::Player),
            AppMessage::Modal(modal::ModalMessage::Import(modal::import::ImportMessage::Cancel))
            | AppMessage::Modal(modal::ModalMessage::Export(modal::export::ExportMessage::Cancel))
            | AppMessage::Modal(modal::ModalMessage::Error(modal::error::ErrorMessage::Acknowledge)) => {
                self.current_modal = modal::Modal::None;
            },
            AppMessage::Modal(modal::ModalMessage::Import(msg)) => {
                if let modal::import::ImportMessage::Confirm = msg {
                    self.current_modal = modal::Modal::None;
                    let import_type = self.import.config.import_type;
                    let text = std::mem::take(&mut self.import.config.text);
                    self.import.update(msg);
                    return self
                        .player
                        .update(PlayerMessage::Import(player::ImportMessage::StartImport(
                            import_type,
                            text,
                        )))
                        .map(AppMessage::Player);
                }
                self.import.update(msg);
            },
            AppMessage::Modal(modal::ModalMessage::Export(msg)) => {
                if let modal::export::ExportMessage::Confirm = msg {
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
        self.player.subscriptions().map(AppMessage::Player)
    }
}
