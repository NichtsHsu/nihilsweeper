pub mod export;
pub mod import;
pub mod error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Modal {
    #[default]
    None,
    ImportGame,
    ExportGame,
    Error,
}


#[derive(Debug, Clone)]
pub enum ModalMessage {
    Import(import::ImportMessage),
    Export(export::ExportMessage),
    Error(error::ErrorMessage),
}