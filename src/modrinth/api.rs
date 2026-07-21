use std::{collections::HashMap, iter::zip, sync::Arc};

use anyhow::Result;
use iced::widget::image::Handle;
use moka::future::Cache;
use reqwest::Client;
use tokio::task::JoinHandle;

use crate::modrinth::types::*;

const MODRINTH_SERVER: &str = "https://api.modrinth.com/v2";

pub enum Status {
    Found(ProjectResponse),
    NotFound(String),
}

pub struct ProjectResponse {
    pub id: &'static str,
    pub url: String,
    pub hash: String,
    pub dep_project_to_version: HashMap<String, Option<String>>,
}

pub async fn check_latest(client: Client, id: &'static str, mc_version: &str) -> Result<Status> {
    let mut modrinth_url = format!("{MODRINTH_SERVER}/project/{id}");
    let project: Project = client
        .get(&modrinth_url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    modrinth_url = format!("{modrinth_url}/version");
    let version_response: Vec<Version> = client
        .get(&modrinth_url)
        .query(&[
            ("loaders", r#"["fabric","quilt"]"#),
            ("game_versions", &format!(r#"["{mc_version}"]"#)),
            ("include_changelog", "false"),
        ])
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    if let Some(version) = version_response.first()
        && let Some(file) = version.files.first()
    {
        let url = file.url.to_owned();
        let hash = file.hashes.sha1.to_owned();
        let mut deps = HashMap::new();
        if let Some(dep_array) = &version.dependencies {
            for dep in dep_array {
                if dep.dependency_type == "required"
                    && let Some(project_id) = &dep.project_id
                {
                    deps.insert(
                        project_id.to_string(),
                        dep.version_id.as_ref().map(|x| x.to_string()),
                    );
                }
            }
        }
        Ok(Status::Found(ProjectResponse {
            id,
            url,
            hash,
            dep_project_to_version: deps,
        }))
    } else {
        Ok(Status::NotFound(project.slug))
    }
}

pub struct DepResponse {
    pub project_id: String,
    pub url: String,
    pub hash: String,
}

pub async fn get_dep_from_version_id(
    client: Client,
    version_id: String,
) -> Result<Option<DepResponse>> {
    let modrinth_url = format!("{MODRINTH_SERVER}/version/{version_id}");
    let version_response: Version = client.get(&modrinth_url).send().await?.json().await?;
    if let Some(file) = version_response.files.first() {
        println!("Found dependency \x1b[35m{version_id}\x1b[39m.");
        return Ok(Some(DepResponse {
            project_id: version_response.project_id,
            url: file.url.to_owned(),
            hash: file.hashes.sha1.to_owned(),
        }));
    };
    Ok(None)
}

pub async fn get_dep_from_project_id(
    client: Client,
    project_id: String,
    mc_version: &str,
) -> Result<Option<DepResponse>> {
    let modrinth_url = format!("{MODRINTH_SERVER}/project/{project_id}/version");
    let version_response: Vec<Version> = client
        .get(&modrinth_url)
        .query(&[("loaders", ["fabric", "quilt"])])
        .query(&[("game_versions", [mc_version])])
        .query(&[("include_changelog", false)])
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    if let Some(version) = version_response.first()
        && let Some(file) = version.files.first()
    {
        println!("Found dependency \x1b[35m{project_id}\x1b[39m.");
        return Ok(Some(DepResponse {
            project_id,
            url: file.url.to_owned(),
            hash: file.hashes.sha1.to_owned(),
        }));
    };
    Ok(None)
}

#[derive(Clone, Debug)]
pub struct SearchResponse {
    pub slug: String,
    pub title: String,
    pub description: String,
    pub icon: Option<Arc<Handle>>,
}

pub async fn search_projects(
    client: Client,
    cache: Cache<String, Arc<Handle>>,
    query: &str,
) -> Result<Vec<SearchResponse>> {
    let modrinth_url = format!("{MODRINTH_SERVER}/search");
    let search_response: SearchResult = client
        .get(&modrinth_url)
        .query(&[("query", query)])
        .query(&[(
            "facets",
            r#"[["categories:fabric", "categories:quilt"], ["project_type:mod"]]"#,
        )])
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let mut projects_without_icon = Vec::new();
    let mut icon_futures: Vec<JoinHandle<Result<Option<Arc<Handle>>, Arc<reqwest::Error>>>> =
        Vec::new();
    for project in search_response.hits {
        let handle = if project.icon_url.is_empty() {
            tokio::spawn(async { Ok(None::<Arc<Handle>>) })
        } else {
            let client = client.clone();
            let cache = cache.clone();
            tokio::spawn(async move {
                Ok(Some(
                    cache
                        .try_get_with(project.icon_url.clone(), async {
                            let bytes = client
                                .get(&project.icon_url)
                                .send()
                                .await?
                                .error_for_status()?
                                .bytes()
                                .await?;
                            Ok::<Arc<Handle>, reqwest::Error>(Arc::new(Handle::from_bytes(bytes)))
                        })
                        .await?,
                ))
            })
        };
        projects_without_icon.push((
            project.slug.clone(),
            project.title.clone(),
            project.description.clone(),
        ));
        icon_futures.push(handle);
    }
    let mut projects = Vec::new();
    for (proj, icon) in zip(projects_without_icon, icon_futures) {
        projects.push(SearchResponse {
            slug: proj.0,
            title: proj.1,
            description: proj.2,
            icon: icon.await??,
        });
    }
    Ok(projects)
}
