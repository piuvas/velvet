#![windows_subsystem = "windows"]

mod get_mods;
mod gui;
mod install_velvet;
pub mod modrinth;
pub mod write_json;

use gui::Velvet;

fn main() -> iced::Result {
    iced::daemon(Velvet::new, Velvet::update, Velvet::view)
        .title(Velvet::title)
        .theme(Velvet::theme)
        .subscription(Velvet::subscription)
        .run()
}
