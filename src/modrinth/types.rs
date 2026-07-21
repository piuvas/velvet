use serde::Deserialize;

#[derive(Deserialize)]
pub(super) struct Project {
    pub(super) slug: String,
}

#[derive(Deserialize)]
pub(super) struct Version {
    pub(super) project_id: String,
    pub(super) files: Vec<VersionFile>,
    pub(super) dependencies: Option<Vec<Dependency>>,
}

#[derive(Deserialize)]
pub(super) struct VersionFile {
    pub(super) url: String,
    pub(super) hashes: Hashes,
}

#[derive(Deserialize)]
pub(super) struct Hashes {
    pub(super) sha1: String,
}

#[derive(Deserialize)]
pub(super) struct Dependency {
    pub(super) project_id: Option<String>,
    pub(super) version_id: Option<String>,
    pub(super) dependency_type: String,
}

#[derive(Deserialize)]
pub(super) struct SearchResult {
    pub(super) hits: Vec<SearchProject>,
}

#[derive(Clone, Debug, Deserialize)]
pub(super) struct SearchProject {
    pub(super) slug: String,
    pub(super) title: String,
    pub(super) description: String,
    pub(super) icon_url: String,
}
