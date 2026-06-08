#![windows_subsystem = "windows"]

mod get_mods;
mod gui;
mod install_velvet;
pub mod write_json;

use gui::Velvet;
use iced::{Size, application, window};

fn main() -> iced::Result {
    application(Velvet::new, Velvet::update, Velvet::view)
        .title(Velvet::title)
        .theme(Velvet::theme)
        .window(window::Settings {
            size: Size::new(500.0, 250.0),
            resizable: false,
            icon: window::icon::from_file_data(include_bytes!("../res/icon.png"), None).ok(),
            ..window::Settings::default()
        })
        .antialiasing(true)
        .run()
}
