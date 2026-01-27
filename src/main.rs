#![allow(unused)]

mod base;
mod config;
mod engine;
mod error;
mod ui;
mod utils;

use ui::App;

fn main() -> crate::error::Result<()> {
    env_logger::init();
    iced::application(App::new, App::update, App::view)
        .theme(App::theme)
        .subscription(App::subscriptions)
        .run()?;

    Ok(())
}
