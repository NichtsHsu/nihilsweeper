#![allow(unused)]

mod base;
mod config;
mod error;
mod ui;
mod utils;
mod engine;

use ui::MainWindow;

fn main() -> crate::error::Result<()> {
    env_logger::init();
    iced::application(MainWindow::new, MainWindow::update, MainWindow::view)
        .subscription(MainWindow::subscriptions)
        .run()?;

    Ok(())
}
