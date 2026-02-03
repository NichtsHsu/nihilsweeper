use crate::base::encode_decode::EncodeType;

#[derive(Clone, Debug)]
pub enum ImportMessage {
    TypeSelected(EncodeType),
    TextEdit(iced::widget::text_editor::Action),
    Confirm,
    Cancel,
}

#[derive(Clone, Debug)]
pub struct ImportConfig {
    pub text: iced::widget::text_editor::Content,
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
                text: iced::widget::text_editor::Content::new(),
                import_type: EncodeType::Base64,
            },
            import_type_selector: iced::widget::combo_box::State::new(EncodeType::DECODE_TYPES.to_vec()),
        }
    }

    pub fn update(&mut self, message: ImportMessage) {
        match message {
            ImportMessage::TypeSelected(import_type) => {
                self.config.import_type = import_type;
            },
            ImportMessage::TextEdit(action) => {
                self.config.text.perform(action);
            },
            _ => _ = std::mem::take(&mut self.config.text),
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
                iced::widget::text_editor(&self.config.text)
                    .on_action(ImportMessage::TextEdit)
                    .font(iced::Font::MONOSPACE)
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
        .width(600)
        .padding(20)
        .style(iced::widget::container::rounded_box)
        .into()
    }
}
