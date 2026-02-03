use crate::base::encode_decode::EncodeType;

#[derive(Clone, Debug)]
pub struct ExportConfig {
    pub export_type: EncodeType,
}

#[derive(Debug, Clone)]
pub enum ExportMessage {
    TypeSelected(EncodeType),
    Confirm,
    Cancel,
}

pub struct ExportModal {
    pub config: ExportConfig,
    export_type_selector: iced::widget::combo_box::State<EncodeType>,
}

impl ExportModal {
    pub fn new() -> Self {
        Self {
            config: ExportConfig {
                export_type: EncodeType::Base64,
            },
            export_type_selector: iced::widget::combo_box::State::new(EncodeType::ENCODE_TYPES.to_vec()),
        }
    }

    pub fn update(&mut self, message: ExportMessage) {
        if let ExportMessage::TypeSelected(export_type) = message {
            self.config.export_type = export_type;
        }
    }

    pub fn view(&self) -> iced::Element<'_, ExportMessage> {
        iced::widget::container(
            iced::widget::column![
                iced::widget::row![
                    iced::widget::text("Export to:").size(20),
                    iced::widget::combo_box(
                        &self.export_type_selector,
                        "Select Export Type",
                        Some(&self.config.export_type),
                        ExportMessage::TypeSelected
                    )
                ]
                .spacing(10),
                iced::widget::row![
                    iced::widget::button(iced::widget::text("Confirm"))
                        .on_press(ExportMessage::Confirm)
                        .padding([10, 20]),
                    iced::widget::button(iced::widget::text("Cancel"))
                        .on_press(ExportMessage::Cancel)
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
