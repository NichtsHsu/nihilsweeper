pub enum AnalyzerMessage {}

pub struct Analyzer {}

impl Analyzer {
    pub fn new() -> Self {
        Self {}
    }

    pub fn update(&mut self) {}

    pub fn view(&self) -> iced::Element<'_, AnalyzerMessage> {
        iced::widget::scrollable("todo").into()
    }
}
