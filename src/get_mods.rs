use anyhow::Result;
use iced::futures::FutureExt;
use iced::futures::future::{Either, try_join_all};
use reqwest::Client;
use sha1_smol::Sha1;
use tokio::fs::{File, read, read_dir, remove_file};
use tokio::io::AsyncWriteExt;

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use crate::modrinth::api::{self, Status};

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
    "4I1XuqiY", // entity-model-features
];

pub async fn run(
    client: Client,
    mc_version: &str,
    modlist: &(bool, bool, bool),
    path_mods: PathBuf,
) -> Result<Vec<String>> {
    let mut existing_hash = HashSet::new();
    let mut delete_ids = HashSet::new();
    let mut mod_folder_reader = read_dir(&path_mods).await?;

    while let Some(file) = mod_folder_reader.next_entry().await? {
        let file_name = file.file_name().to_string_lossy().to_string();
        let file_bytes = read(file.path()).await?;
        let file_hash = Sha1::from(file_bytes).hexdigest();

        let mod_id = String::from(&file_name[0..8]);

        delete_ids.insert(mod_id);
        existing_hash.insert(file_hash);
    }

    let mut selected_id_set = HashSet::new();

    if modlist.0 {
        for x in VANILLA {
            selected_id_set.insert(x);
        }
    }
    if modlist.1 {
        for x in VISUAL {
            selected_id_set.insert(x);
        }
    }
    if modlist.2 {
        for x in OPTIFINE {
            selected_id_set.insert(x);
        }
    }

    // in the first batch, we check all project ids to see available mods and fetch dependencies.
    // we also check if they already exist in the mod folder by comparing the sha1 hash.

    println!("1");

    let mut check_latest_futures = Vec::new();
    for id in selected_id_set {
        check_latest_futures.push(api::check_latest(client.clone(), id, mc_version));
    }

    let mut not_found = Vec::new();
    let mut selected_id_to_url_hash = HashMap::new();

    let mut get_dep_futures: Vec<Either<_, _>> = Vec::new();
    for result in try_join_all(check_latest_futures).await? {
        match result {
            Status::Found(response) => {
                let id = response.id;
                let hash = response.hash;
                let url = response.url;
                let deps = response.dep_project_to_version;

                if existing_hash.contains(&hash) {
                    println!("Already found \x1b[35m{id}\x1b[39m.");
                    delete_ids.remove(id);
                } else {
                    selected_id_to_url_hash.insert(id, (url, hash));
                }
                for (dep_project_id, dep_version_id) in deps {
                    if let Some(dep_version_id) = dep_version_id {
                        get_dep_futures.push(
                            api::get_dep_from_version_id(client.clone(), dep_version_id)
                                .left_future(),
                        );
                    } else {
                        get_dep_futures.push(
                            api::get_dep_from_project_id(
                                client.clone(),
                                dep_project_id,
                                mc_version,
                            )
                            .right_future(),
                        );
                    }
                }
            }
            Status::NotFound(name) => not_found.push(name),
        }
    }

    // in the second batch, we fetch the url and hash for all dependencies.

    let mut dep_id_to_url_hash = HashMap::new();
    for dep in try_join_all(get_dep_futures).await?.into_iter().flatten() {
        let id = dep.project_id;
        let url = dep.url;
        let hash = dep.hash;

        if existing_hash.contains(&hash) {
            delete_ids.remove(&id);
        } else {
            dep_id_to_url_hash.insert(id, (url, hash));
        }
    }

    // then, we make sure dependencies with strict versions don't conflict with selected mods.

    for id in dep_id_to_url_hash.keys() {
        selected_id_to_url_hash.remove(id.as_str());
    }

    // in the third batch, we download all mods

    let mut download_mod_futures = Vec::new();
    for (id, (url, _)) in selected_id_to_url_hash {
        download_mod_futures.push(download_mod(
            client.clone(),
            url,
            id.to_owned(),
            path_mods.clone(),
        ));
    }
    for (id, (url, _)) in dep_id_to_url_hash {
        download_mod_futures.push(download_mod(client.clone(), url, id, path_mods.clone()));
    }

    for id in try_join_all(download_mod_futures).await? {
        delete_ids.remove(&id);
    }

    // then, we delete files which were neither just created nor verified to be selected.

    let mut delete_file_futures = Vec::new();
    for id in delete_ids {
        println!("Removing \x1b[35m{id}\x1b[39m.");
        delete_file_futures.push(remove_file(path_mods.join(id).with_extension("jar")));
    }
    try_join_all(delete_file_futures).await?;

    Ok(not_found)
}

async fn download_mod(client: Client, url: String, id: String, path: PathBuf) -> Result<String> {
    println!("Downloading \x1b[35m{id}\x1b[39m.");
    let path = path.join(&id).with_extension("jar");
    println!("2: {url}");
    let download = client.get(url).send().await?.bytes().await?;
    let mut mod_file = File::create(path).await?;
    mod_file.write_all(&download).await?;
    println!("Finished downloading \x1b[35m{id}\x1b[39m.");
    Ok(id)
}
