#![allow(unused)]

mod base;
mod config;
mod engine;
mod error;
mod single_instance;
mod ui;
mod utils;

use log::info;
use ui::App;

fn main() -> crate::error::Result<()> {
    env_logger::init();

    // Check for single instance
    match single_instance::check_single_instance() {
        Ok(true) => {
            info!("Starting as the first instance");
            iced::application(App::new, App::update, App::view)
                .window(iced::window::Settings {
                    exit_on_close_request: false,
                    ..Default::default()
                })
                .theme(App::theme)
                .subscription(App::subscriptions)
                .run()?;
        },
        Ok(false) => {
            info!("Another instance is already running, exiting");
            // Exit silently - the other instance has been notified
        },
        Err(e) => {
            eprintln!("Error checking single instance: {}", e);
            // Continue anyway to not break the application
            iced::application(App::new, App::update, App::view)
                .window(iced::window::Settings {
                    exit_on_close_request: false,
                    ..Default::default()
                })
                .theme(App::theme)
                .subscription(App::subscriptions)
                .run()?;
        },
    }

    Ok(())
}
