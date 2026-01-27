use iced::Task;
use log::warn;

use crate::ui::{
    main_window::{MainWindow, MainWindowMessage},
    *,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Modal {
    #[default]
    MainWindow,
    ImportGame,
}

#[derive(Debug, Clone)]
pub enum ModalMessage {
    TextChange(String),
    Submit,
    Close,
}

#[derive(Debug, Clone)]
pub enum AppMessage {
    Modal(ModalMessage),
    MainWindow(main_window::MainWindowMessage),
}

pub struct App {
    current_modal: Modal,
    main_window: main_window::MainWindow,
    modal_text_input: String,
}

impl App {
    pub fn new() -> Self {
        Self {
            current_modal: Modal::default(),
            main_window: main_window::MainWindow::new(),
            modal_text_input: String::default(),
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
            AppMessage::MainWindow(MainWindowMessage::ShowImportModal) => {
                self.current_modal = Modal::ImportGame;
            },
            AppMessage::MainWindow(msg) => return self.main_window.update(msg).map(AppMessage::MainWindow),
            AppMessage::Modal(msg) => match msg {
                ModalMessage::TextChange(new_text) => {
                    self.modal_text_input = new_text;
                },
                ModalMessage::Submit => {
                    let modal = std::mem::replace(&mut self.current_modal, Modal::MainWindow);
                    let text = std::mem::take(&mut self.modal_text_input);
                    match modal {
                        Modal::ImportGame => {
                            return self
                                .main_window
                                .update(MainWindowMessage::Import(main_window::ImportMessage::StartImport(text)))
                                .map(AppMessage::MainWindow);
                        },
                        Modal::MainWindow => {
                            warn!("ModalMessage::Submit should not be received in MainWindow modal.");
                        },
                    }
                },
                ModalMessage::Close => {
                    self.current_modal = Modal::MainWindow;
                    self.modal_text_input.clear();
                },
            },
        }
        Task::none()
    }

    pub fn view(&self) -> iced::Element<'_, AppMessage> {
        match self.current_modal {
            Modal::ImportGame => {
                let container = iced::widget::container(
                    iced::widget::column![
                        iced::widget::text("Please input the base64 code").size(20),
                        iced::widget::text_input("Please input here...", &self.modal_text_input)
                            .on_input(|text| AppMessage::Modal(ModalMessage::TextChange(text)))
                            .on_submit(AppMessage::Modal(ModalMessage::Submit))
                            .padding(10),
                        iced::widget::row![
                            iced::widget::button(iced::widget::text("Confirm"))
                                .on_press(AppMessage::Modal(ModalMessage::Submit))
                                .padding([10, 20]),
                            iced::widget::button(iced::widget::text("Cancel"))
                                .on_press(AppMessage::Modal(ModalMessage::Close))
                                .padding([10, 20]),
                        ]
                        .spacing(10)
                    ]
                    .spacing(15),
                )
                .width(400)
                .padding(20)
                .style(iced::widget::container::rounded_box);
                App::modal(
                    self.main_window.view().map(AppMessage::MainWindow),
                    container,
                    AppMessage::Modal(ModalMessage::Close),
                )
            },
            Modal::MainWindow => self.main_window.view().map(AppMessage::MainWindow),
        }
    }

    pub fn theme(&self) -> Option<iced::Theme> {
        self.main_window.theme()
    }

    pub fn subscriptions(&self) -> iced::Subscription<AppMessage> {
        self.main_window.subscriptions().map(AppMessage::MainWindow)
    }
}
