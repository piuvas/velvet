mod theme;
mod views;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use iced::theme::Palette;
use iced::widget::Column;
use iced::widget::image::Handle;
use iced::window::Id;
use iced::{Function, Subscription, Theme, task, window};
use iced::{Task, futures::TryFutureExt};
use moka::future::Cache;
use reqwest::{Client, ClientBuilder};
use rfd::{AsyncFileDialog, FileHandle};
use serde::Deserialize;
use sha1_smol::Sha1;
use tokio::time::sleep;

use crate::gui::views::main::ADDED_HEIGHT;
use crate::gui::views::{Icons, WindowType};
use crate::modrinth::api::{self, SearchResponse};
use crate::{get_mods, install_velvet};

pub struct Velvet {
    client: Client,
    windows: HashMap<Id, WindowType>,
    icons: Icons,

    version_list: Vec<String>,
    snapshot: bool,
    version: Option<String>,
    vanilla: bool,
    beauty: bool,
    optifine: bool,
    modrinth_query: String,
    modrinth_query_abort: Option<task::Handle>,
    modrinth_query_results: Option<Result<Vec<SearchResponse>, String>>,
    image_cache: Cache<String, Arc<Handle>>,
    extra_mods: Vec<ExtraMod>,
    status: Status,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExtraMod {
    title: String,
    id: String,
}

enum Status {
    Idle,
    Installing,
    NoVersion,
    Success(Vec<String>),
    Failure(String),
}

#[derive(Debug, Clone)]
pub enum Message {
    SetWindowType(WindowType, Id),
    WindowOpened(Id),
    WindowClosed(Id),

    PopulateMcVersions(Vec<String>),
    UpdateVersion(String),
    Snapshot(bool),
    VButton(bool),
    BButton(bool),
    OButton(bool),

    OpenExtraWindow,

    OpenImportDialog,
    OpenExportDialog,
    ImportExtraJson(Option<FileHandle>),
    ExportExtraJson(Option<FileHandle>),
    TryAddExtraMods(Result<Vec<ExtraMod>, String>),
    TryWriteExtraMods(Result<(), String>),

    UpdatedQuery(String),
    SearchModrinth(String),
    PopulateSearchResults(Result<Vec<SearchResponse>, String>),

    AddExtraMod(ExtraMod),
    RemoveExtraMod(usize),

    Install,
    Done(Result<Vec<String>, String>),
}

impl Velvet {
    pub fn new() -> (Self, Task<Message>) {
        let (id, open) = views::open(WindowType::Main);
        let client = ClientBuilder::new()
            .user_agent(concat!(
                env!("CARGO_PKG_NAME"),
                "+",
                env!("CARGO_PKG_VERSION")
            ))
            .build()
            .expect("Couldn't build reqwest client.");
        let cache = Cache::builder()
            .max_capacity(128)
            .time_to_idle(Duration::from_secs(30))
            .build();
        (
            Velvet {
                client: client.clone(),
                windows: HashMap::new(),
                icons: Icons::new(),
                version_list: Vec::new(),
                snapshot: false,
                version: None,
                vanilla: true,
                beauty: false,
                optifine: false,
                modrinth_query: String::new(),
                modrinth_query_abort: None,
                modrinth_query_results: None,
                image_cache: cache,
                extra_mods: Vec::new(),
                status: Status::Idle,
            },
            Task::batch([
                Task::perform(populate(client, false), Message::PopulateMcVersions),
                Task::done(id)
                    .map(Message::SetWindowType.with(WindowType::Main))
                    .chain(open.map(Message::WindowOpened)),
            ]),
        )
    }

    pub fn title(&self, id: Id) -> String {
        match (self.windows.get(&id), &self.version) {
            (Some(WindowType::Main), Some(value)) => format!("Velvet Installer - {}", &value),
            (Some(WindowType::Extra), _) => String::from("Velvet Installer - Mod Search"),
            _ => String::from("Velvet Installer"),
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SetWindowType(window_type, id) => {
                self.windows.insert(id, window_type);
            }
            Message::WindowOpened(id) => {
                if !self.windows.contains_key(&id) {
                    return iced::exit();
                }
            }
            Message::WindowClosed(id) => {
                if let Some(window_type) = self.windows.get(&id) {
                    if window_type == &WindowType::Main {
                        return iced::exit();
                    } else {
                        self.windows.remove(&id);
                        self.modrinth_query.clear();
                        self.modrinth_query_results = None;
                    }
                }
            }
            Message::PopulateMcVersions(value) => {
                self.version = Some(value[0].clone());
                self.version_list = value;
            }
            Message::UpdateVersion(value) => self.version = Some(value),
            Message::Snapshot(value) => {
                self.snapshot = value;
                return Task::perform(
                    populate(self.client.clone(), value),
                    Message::PopulateMcVersions,
                );
            }
            Message::VButton(value) => self.vanilla = value,
            Message::BButton(value) => self.beauty = value,
            Message::OButton(value) => self.optifine = value,

            Message::OpenExtraWindow => {
                let (id, open) = views::open(WindowType::Extra);
                for window_type in self.windows.values() {
                    if window_type == &WindowType::Extra {
                        return Task::none();
                    }
                }
                return Task::done(id)
                    .map(Message::SetWindowType.with(WindowType::Extra))
                    .chain(open.map(Message::WindowOpened));
            }
            Message::OpenImportDialog => {
                return Task::perform(
                    AsyncFileDialog::new()
                        .set_title("Select mod list file:")
                        .add_filter("Mod list", &["json"])
                        .pick_file(),
                    Message::ImportExtraJson,
                );
            }
            Message::OpenExportDialog => {
                let mut hash = Sha1::new();
                self.extra_mods
                    .iter()
                    .for_each(|extra_mod| hash.update(extra_mod.id.as_bytes()));
                let hex = hash.hexdigest();
                return Task::perform(
                    AsyncFileDialog::new()
                        .set_title("Save mod list file:")
                        .set_file_name(format!("velvet-mods-{}.json", &hex[0..8]))
                        .save_file(),
                    Message::ExportExtraJson,
                );
            }
            Message::ImportExtraJson(value) => {
                if let Some(file_handle) = value {
                    let client = self.client.clone();
                    let future = async move {
                        let bytes = file_handle.read().await;
                        let ids: Vec<String> = serde_json::from_slice(&bytes)?;
                        get_extra_mods_from_ids(client, ids).await
                    };
                    return Task::perform(
                        future.map_err(|err| err.to_string()),
                        Message::TryAddExtraMods,
                    );
                }
            }
            Message::ExportExtraJson(value) => {
                if let Some(file_handle) = value {
                    let extra_mods = self.extra_mods.clone();
                    let future = async move {
                        let bytes = serde_json::to_vec_pretty::<Vec<String>>(
                            &extra_mods
                                .into_iter()
                                .map(|extra_mod| extra_mod.id)
                                .collect(),
                        )?;
                        file_handle.write(bytes.as_slice()).await?;
                        Ok::<(), anyhow::Error>(())
                    };
                    return Task::perform(
                        future.map_err(|err| err.to_string()),
                        Message::TryWriteExtraMods,
                    );
                }
            }
            Message::TryAddExtraMods(value) => match value {
                Ok(new_extra_mods) => {
                    for new_extra_mod in new_extra_mods {
                        if !self.extra_mods.contains(&new_extra_mod) {
                            self.extra_mods.push(new_extra_mod);
                        }
                    }
                }
                Err(err) => self.status = Status::Failure(err),
            },
            Message::TryWriteExtraMods(value) => {
                if let Err(err) = value {
                    self.status = Status::Failure(err);
                }
            }

            Message::UpdatedQuery(value) => {
                self.modrinth_query = value.clone();
                if let Some(task) = self.modrinth_query_abort.take() {
                    task.abort();
                }
                let (task_handle, abort_handle) = Task::perform(
                    async {
                        sleep(Duration::from_millis(300)).await;
                        value
                    },
                    Message::SearchModrinth,
                )
                .abortable();
                self.modrinth_query_abort = Some(abort_handle);
                return task_handle;
            }
            Message::SearchModrinth(value) => {
                return Task::perform(
                    search_modrinth(self.client.clone(), self.image_cache.clone(), value),
                    Message::PopulateSearchResults,
                );
            }
            Message::PopulateSearchResults(value) => self.modrinth_query_results = Some(value),

            Message::AddExtraMod(value) => {
                if !self.extra_mods.contains(&value) {
                    self.extra_mods.push(value)
                }
            }
            Message::RemoveExtraMod(value) => _ = self.extra_mods.remove(value),

            Message::Install => {
                match &self.version {
                    Some(value) => {
                        self.status = Status::Installing;
                        let values = (self.vanilla, self.beauty, self.optifine);
                        let extra_mods = self
                            .extra_mods
                            .clone()
                            .into_iter()
                            .map(|extra_mod| extra_mod.id)
                            .collect();
                        let mut tasks = Vec::new();
                        tasks.push(Task::perform(
                            run(self.client.clone(), value.clone(), values, extra_mods).map_err(
                                |err| {
                                    eprintln!("{err:#?}");
                                    err.to_string()
                                },
                            ),
                            Message::Done,
                        ));
                        tasks.push(window::oldest().and_then(move |id| {
                            window::resize(id, (500.0, views::main::WINDOW_HEIGHT).into())
                        }));
                        return Task::batch(tasks);
                    }
                    None => self.status = Status::NoVersion,
                };
            }
            Message::Done(value) => match value {
                Ok(x) => {
                    let missing_mods = !x.is_empty();
                    self.status = Status::Success(x);
                    if missing_mods {
                        return window::oldest().and_then(move |id| {
                            window::resize(
                                id,
                                (500.0, views::main::WINDOW_HEIGHT + ADDED_HEIGHT).into(),
                            )
                        });
                    };
                }
                Err(e) => {
                    self.status = Status::Failure(e);
                    return window::oldest().and_then(move |id| {
                        window::resize(
                            id,
                            (500.0, views::main::WINDOW_HEIGHT + ADDED_HEIGHT).into(),
                        )
                    });
                }
            },
        }
        Task::none()
    }

    pub fn view(&self, id: Id) -> Column<'_, Message> {
        if let Some(window_type) = self.windows.get(&id) {
            match window_type {
                WindowType::Main => views::main::view(self),
                WindowType::Extra => views::extra::view(self),
            }
        } else {
            Column::new()
        }
    }

    pub fn theme(&self, _: Id) -> Theme {
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

    pub fn subscription(&self) -> Subscription<Message> {
        window::close_events().map(Message::WindowClosed)
    }
}

#[derive(Deserialize)]
struct Response {
    version: String,
}

async fn run(
    client: Client,
    mc_version: String,
    modlists: (bool, bool, bool),
    extra_mods: Vec<String>,
) -> Result<Vec<String>> {
    let response: Vec<Response> = client
        .get("https://meta.quiltmc.org/v3/versions/loader")
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let quilt_version = &response[0].version;

    let path_mods = install_velvet::run(client.clone(), &mc_version, quilt_version).await?;
    let missing = get_mods::run(client, &mc_version, &modlists, extra_mods, path_mods).await?;
    Ok(missing)
}

#[derive(Deserialize)]
struct Versions {
    version: String,
    stable: bool,
}

async fn populate(client: Client, snapshots: bool) -> Vec<String> {
    let mut versions_list = Vec::new();
    let response: Vec<Versions> = client
        .get("https://meta.quiltmc.org/v3/versions/game")
        .send()
        .await
        .expect("Couldn't get versions.")
        .error_for_status()
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
async fn search_modrinth(
    client: Client,
    cache: Cache<String, Arc<Handle>>,
    query: String,
) -> Result<Vec<SearchResponse>, String> {
    api::search_projects(client, cache, &query)
        .await
        .map_err(|err| err.to_string())
}

async fn get_extra_mods_from_ids(client: Client, ids: Vec<String>) -> Result<Vec<ExtraMod>> {
    let projects = api::get_projects_from_ids(client, ids.clone()).await?;
    Ok(ids
        .into_iter()
        .zip(projects)
        .map(|(id, project)| ExtraMod {
            title: project.title,
            id,
        })
        .collect())
}
