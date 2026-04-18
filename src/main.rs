#![windows_subsystem = "windows"]

mod get_minecraft_dir;
mod get_mods;
mod install_velvet;
mod theme;
pub mod write_json;

use anyhow::Result;
use iced::futures::TryFutureExt;
use iced::widget::{Column, button, checkbox, column, container, pick_list, space, text, tooltip};
use iced::{Alignment, Element, Length, Size, Task, Theme, application, theme::Palette, window};
use serde::Deserialize;

#[derive(Deserialize)]
struct Versions {
    version: String,
    stable: bool,
}

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

struct Velvet {
    version_list: Vec<String>,
    snapshot: bool,
    version: Option<String>,
    vanilla: bool,
    beauty: bool,
    optifine: bool,
    status: Status,
}

enum Status {
    Idle,
    Installing,
    NoVersion,
    Success(Vec<String>),
    Failure(String),
}

#[derive(Debug, Clone)]
enum Message {
    Populate(Vec<String>),
    Update(String),
    Snapshot(bool),
    VButton(bool),
    BButton(bool),
    OButton(bool),
    Pressed,
    Done(Result<Vec<String>, String>),
}

impl Velvet {
    fn new() -> (Self, Task<Message>) {
        (
            Velvet {
                version_list: Vec::new(),
                snapshot: false,
                version: None,
                vanilla: true,
                beauty: false,
                optifine: false,
                status: Status::Idle,
            },
            Task::perform(populate(false), Message::Populate),
        )
    }

    fn title(&self) -> String {
        match &self.version {
            None => String::from("Velvet Installer"),
            Some(value) => format!("Velvet Installer - {}", &value),
        }
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Populate(value) => {
                self.version = Some(value[0].clone());
                self.version_list = value;
            }
            Message::Update(value) => self.version = Some(value),
            Message::Snapshot(value) => {
                self.snapshot = value;
                return Task::perform(populate(value), Message::Populate);
            }
            Message::VButton(value) => self.vanilla = value,
            Message::BButton(value) => self.beauty = value,
            Message::OButton(value) => self.optifine = value,
            Message::Pressed => {
                match &self.version {
                    Some(value) => {
                        self.status = Status::Installing;
                        let values = (self.vanilla, self.beauty, self.optifine);
                        let mut commands = Vec::new();
                        commands.push(Task::perform(
                            run(value.clone(), values).map_err(|e| {
                                eprintln!("{e:#?}");
                                format!("{e}")
                            }),
                            Message::Done,
                        ));
                        commands.push(
                            window::oldest()
                                .and_then(move |id| window::resize(id, (500.0, 250.0).into())),
                        );
                        return Task::batch(commands);
                    }
                    None => self.status = Status::NoVersion,
                };
            }
            Message::Done(value) => match value {
                Ok(x) => {
                    let missing_mods = !x.is_empty();
                    self.status = Status::Success(x);
                    if missing_mods {
                        return window::oldest()
                            .and_then(move |id| window::resize(id, (500.0, 275.0).into()));
                    };
                }
                Err(e) => {
                    self.status = Status::Failure(e);
                    return window::oldest()
                        .and_then(move |id| window::resize(id, (500.0, 275.0).into()));
                }
            },
        }
        Task::none()
    }

    fn view(&self) -> Column<'_, Message> {
        let (button_message, extra_message): (&str, Option<Element<Message>>) = match &self.status {
            Status::Idle => ("Install", None),
            Status::Installing => ("Installing...", None),
            Status::NoVersion => ("No version selected!", None),
            Status::Success(mods) => (
                "Finished!",
                if !mods.is_empty() {
                    Some({
                        let mut mod_string = String::new();
                        mod_string.push_str(&mods[0]);
                        for name in mods.iter().skip(1) {
                            mod_string.push_str(", ");
                            mod_string.push_str(name);
                        }
                        tooltip(
                            "Hover to see unavailable mods.",
                            container(text(mod_string)),
                            tooltip::Position::FollowCursor,
                        )
                        .style(theme::container_style)
                        .into()
                    })
                } else {
                    None
                },
            ),
            Status::Failure(e) => ("Error!", Some(text(e).color(theme::LOVE).into())),
        };

        let mut main_column = column![
            text("Enter Minecraft version:").size(20),
            pick_list(
                self.version_list.clone(),
                self.version.clone(),
                Message::Update
            )
            .placeholder("Loading...")
            .width(Length::Fixed(200.0))
            .style(theme::pick_list_style)
            .menu_style(theme::menu_style),
            space().height(Length::Fixed(10.0)),
            checkbox(self.snapshot)
                .label("Show snapshots")
                .on_toggle(Message::Snapshot)
                .style(theme::checkbox_style),
            column![
                checkbox(self.vanilla)
                    .label("Vanilla - Performance enhancing modlist.")
                    .on_toggle(Message::VButton)
                    .style(theme::checkbox_style),
                checkbox(self.beauty)
                    .label("Beauty - Immersive and beautiful modlist.")
                    .on_toggle(Message::BButton)
                    .style(theme::checkbox_style),
                checkbox(self.optifine)
                    .label("Optifine - Optifine resource pack parity.")
                    .on_toggle(Message::OButton)
                    .style(theme::checkbox_style),
            ]
            .align_x(Alignment::Start),
            space().height(Length::Fixed(10.0)),
            button(button_message)
                .on_press(Message::Pressed)
                .style(theme::button_style),
        ]
        .spacing(5.0)
        .padding(10.0)
        .align_x(Alignment::Center)
        .width(Length::Fill)
        .height(Length::Fill);

        if let Some(message) = extra_message {
            main_column = main_column.push(space().height(Length::Fill));
            main_column = main_column.push(message);
        }

        main_column
    }

    fn theme(&self) -> Theme {
        Theme::custom(
            "Rosé Pine".to_string(),
            Palette {
                background: theme::BASE,
                text: theme::TEXT,
                primary: theme::LOVE,
                success: theme::FOAM,
                danger: theme::LOVE,
                warning: theme::GOLD,
            },
        )
    }
}

#[derive(Deserialize)]
struct Response {
    version: String,
}

async fn run(mc_version: String, modlists: (bool, bool, bool)) -> Result<Vec<String>> {
    let response: Vec<Response> = reqwest::get("https://meta.quiltmc.org/v3/versions/loader")
        .await?
        .json()
        .await?;

    let quilt_version = &response[0].version;

    let path_mods = install_velvet::run(&mc_version, quilt_version).await?;
    let missing = get_mods::run(mc_version, &modlists, path_mods).await?;
    Ok(missing)
}

async fn populate(snapshots: bool) -> Vec<String> {
    let mut versions_list = Vec::new();
    let response: Vec<Versions> = reqwest::get("https://meta.quiltmc.org/v3/versions/game")
        .await
        .expect("Couldn't get versions.")
        .json()
        .await
        .unwrap();

    for value in response {
        if snapshots || value.stable {
            versions_list.push(value.version)
        }
    }
    versions_list
}
