use crate::base::board::EncodeType;

#[derive(Clone, Debug)]
pub enum ImportMessage {
    TypeSelected(EncodeType),
    TextChanged(String),
    Confirm,
    Cancel,
}

#[derive(Clone, Debug)]
pub struct ImportConfig {
    pub text: String,
    pub import_type: EncodeType,
}

pub struct ImportModal {
    pub config: ImportConfig,
    import_type_selector: iced::widget::combo_box::State<EncodeType>,
}

impl ImportModal {
    pub fn new() -> Self {
        Self {
            config: ImportConfig {
                text: String::new(),
                import_type: EncodeType::Base64,
            },
            import_type_selector: iced::widget::combo_box::State::new(EncodeType::ALL.to_vec()),
        }
    }

    pub fn update(&mut self, message: ImportMessage) {
        match message {
            ImportMessage::TypeSelected(import_type) => {
                self.config.import_type = import_type;
            },
            ImportMessage::TextChanged(text) => {
                self.config.text = text;
            },
            _ => self.config.text.clear(),
        }
    }

    pub fn view(&self) -> iced::Element<'_, ImportMessage> {
        iced::widget::container(
            iced::widget::column![
                iced::widget::row![
                    iced::widget::text("Import from:").size(20),
                    iced::widget::combo_box(
                        &self.import_type_selector,
                        "Select Import Type",
                        Some(&self.config.import_type),
                        ImportMessage::TypeSelected
                    )
                ]
                .spacing(10),
                iced::widget::text_input("Please input here...", &self.config.text)
                    .on_input(ImportMessage::TextChanged)
                    .on_submit(ImportMessage::Confirm)
                    .padding(10),
                iced::widget::row![
                    iced::widget::button(iced::widget::text("Confirm"))
                        .on_press(ImportMessage::Confirm)
                        .padding([10, 20]),
                    iced::widget::button(iced::widget::text("Cancel"))
                        .on_press(ImportMessage::Cancel)
                        .padding([10, 20]),
                ]
                .spacing(10)
            ]
            .spacing(15),
        )
        .width(400)
        .padding(20)
        .style(iced::widget::container::rounded_box)
        .into()
    }
}
