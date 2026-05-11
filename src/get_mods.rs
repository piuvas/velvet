use anyhow::Result;
use iced::futures::future::try_join_all;
use reqwest::{Client, ClientBuilder};
use serde::Deserialize;
use sha1_smol::Sha1;
use tokio::fs::{File, read, read_dir, remove_file};
use tokio::io::AsyncWriteExt;

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

const VANILLA: [&str; 9] = [
    "AANobbMI", // sodium
    "gvQqBUqZ", // lithium
    "hEOCdOgW", // phosphor
    "hvFnDODi", // lazydfu
    "uXXizFIs", // ferrite-core
    "fQEb0iXm", // krypton
    "5ZwdcRci", // immediatelyfast
    "VSNURh3q", // c2me-fabric
    "KuNKN7d2", // noisium
];

const VISUAL: [&str; 10] = [
    "pcPXJeZi", // effective
    "yBW8D80W", // lambdynamiclights
    "MPCX6s5C", // not-enough-animations
    "mfzaZK3Z", // ears
    "Orvt0mRa", // indium
    "2Uev7LdA", // lambdabettergrass
    "1IjD5062", // continuity
    "YL57xq9U", // iris
    "fxxUqruK", // voxy
    "xT0lnNE9", // voxy-worldgen
];

const OPTIFINE: [&str; 9] = [
    "3IuO68q1", // puzzle
    "PRN43VSY", // animatica
    "Orvt0mRa", // indium
    "iG6ZHsUV", // cull-less-leaves
    "1IjD5062", // continuity
    "2Uev7LdA", // lambdabettergrass
    "otVJckYQ", // cit-resewn
    "BVzZfTc1", // entitytexturefeatures
    "4I1XuqiY", //entity-model-features
];

const MODRINTH_SERVER: &str = "https://api.modrinth.com/v2";

enum Status {
    Found(&'static str, String, String, HashMap<String, String>),
    NotFound(String),
}

#[derive(Deserialize)]
struct Project {
    slug: String,
}

#[derive(Deserialize)]
struct Version {
    files: Vec<VersionFile>,
    dependencies: Option<Vec<Dependency>>,
}

#[derive(Deserialize)]
struct VersionFile {
    url: String,
    hashes: Hashes,
}

#[derive(Deserialize)]
struct Hashes {
    sha1: String,
}

#[derive(Deserialize)]
struct Dependency {
    project_id: Option<String>,
    version_id: Option<String>,
    dependency_type: String,
}

pub async fn run(
    mc_version: String,
    modlist: &(bool, bool, bool),
    path_mods: PathBuf,
) -> Result<Vec<String>> {
    let client = ClientBuilder::new()
        .user_agent(concat!(
            env!("CARGO_PKG_NAME"),
            "+",
            env!("CARGO_PKG_VERSION")
        ))
        .build()?;

    let mut existing_mods = HashMap::new();
    let mut mod_folder_reader = read_dir(&path_mods).await?;

    while let Some(file) = mod_folder_reader.next_entry().await? {
        let file_name = file.file_name().to_string_lossy().to_string();
        let file_bytes = read(file.path()).await?;
        let file_hash = Sha1::from(file_bytes).hexdigest();

        let mod_id = String::from(&file_name[0..8]);

        existing_mods.insert(mod_id, file_hash);
    }

    let mut mods_from_modlist = HashSet::new();

    if modlist.0 {
        for x in VANILLA {
            mods_from_modlist.insert(x);
        }
    }
    if modlist.1 {
        for x in VISUAL {
            mods_from_modlist.insert(x);
        }
    }
    if modlist.2 {
        for x in OPTIFINE {
            mods_from_modlist.insert(x);
        }
    }

    let mut get_versions = Vec::new();

    for x in mods_from_modlist {
        let task = check_latest(x, client.clone(), mc_version.clone());
        get_versions.push(task);
    }

    let mut new_mod_hashes = HashMap::new();
    let mut dependency_list = HashMap::new();
    let mut download_list = HashMap::new();
    let mut mods_not_found = Vec::new();

    for result in try_join_all(get_versions).await? {
        match result {
            Status::NotFound(x) => mods_not_found.push(x),
            Status::Found(id, url, hash, deps) => {
                if let Some(x) = existing_mods.get(id)
                    && x == &hash
                {
                    println!("Already found \x1b[35m{id}\x1b[39m.")
                } else {
                    download_list.insert(id.to_string(), (url, hash));
                }
                for dep in deps {
                    if let Some((dep_url, dep_hash)) =
                        get_files_from_version_id(&dep.1, client.clone()).await?
                    {
                        dependency_list.insert(dep.0, (dep_url, dep_hash));
                    }
                }
            }
        }
    }

    let mut download_mods = Vec::new();

    for (id, (url, hash)) in dependency_list {
        download_list.insert(id, (url, hash));
    }

    for (id, (url, hash)) in download_list {
        download_mods.push(download_mod(
            url,
            id.clone(),
            path_mods.clone(),
            client.clone(),
        ));
        new_mod_hashes.insert(id, hash);
    }

    for (id, hash) in existing_mods {
        if new_mod_hashes.get(&id) != Some(&hash) {
            println!("Removing \x1b[35m{id}\x1b[39m.");
            remove_file(path_mods.join(id).with_extension("jar")).await?
        }
    }

    try_join_all(download_mods).await?;
    Ok(mods_not_found)
}

async fn download_mod(url: String, file_name: String, path: PathBuf, client: Client) -> Result<()> {
    println!("Downloading \x1b[35m{file_name}\x1b[39m.");
    let path = path.join(&file_name).with_extension("jar");
    let download = client.get(url).send().await?.bytes().await?;
    let mut mod_file = File::create(path).await?;
    mod_file.write_all(&download).await?;
    println!("Finished downloading \x1b[35m{file_name}\x1b[39m.");
    Ok(())
}

async fn check_latest(x: &'static str, client: Client, mc_version: String) -> Result<Status> {
    let mut modrinth_url = format!("{MODRINTH_SERVER}/project/{x}");
    let project: Project = client.get(&modrinth_url).send().await?.json().await?;

    modrinth_url = format!(
        "{modrinth_url}/version?loaders=[\"fabric\", \"quilt\"]&game_versions=[{mc_version:?}]&include_changelog=false"
    );

    let version_response: Vec<Version> = client.get(&modrinth_url).send().await?.json().await?;
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
                    && let Some(version_id) = &dep.version_id
                {
                    deps.insert(project_id.to_string(), version_id.to_string());
                }
            }
        }
        Ok(Status::Found(x, url, hash, deps))
    } else {
        Ok(Status::NotFound(project.slug))
    }
}

async fn get_files_from_version_id(id: &str, client: Client) -> Result<Option<(String, String)>> {
    let modrinth_url = format!("{MODRINTH_SERVER}/version/{id}");
    let version_response: Version = client.get(&modrinth_url).send().await?.json().await?;
    if let Some(file) = version_response.files.first() {
        return Ok(Some((file.url.to_owned(), file.hashes.sha1.to_owned())));
    };
    Ok(None)
}
