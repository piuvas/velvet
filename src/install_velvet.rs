use anyhow::{Result, anyhow};
use home::home_dir;
use rfd::AsyncFileDialog;
use tokio::fs;
use tokio::fs::{File, create_dir_all};
use tokio::io::AsyncWriteExt;

use std::path::PathBuf;

use crate::write_json;

pub async fn run(mc_version: &String, quilt_version: &String) -> Result<PathBuf> {
    let mut mc_path: PathBuf = get_minecraft_dir()?;
    while !mc_path.is_dir() {
        mc_path = std::path::PathBuf::from(
            AsyncFileDialog::new()
                .set_title("Select .minecraft folder:")
                .pick_folder()
                .await
                .ok_or_else(|| anyhow!("Select a folder!"))?,
        );
    }

    let velvet_path = PathBuf::from(&mc_path).join(".velvet");
    let path_mods = PathBuf::from(&velvet_path).join("mods").join(mc_version);

    create_dir_all(&path_mods).await?;

    let version_folder_name = format!("quilt-loader-{}-{}", &quilt_version, &mc_version);
    let mut path_version = PathBuf::from(&mc_path)
        .join("versions")
        .join(&version_folder_name);
    create_dir_all(&path_version).await?;
    path_version.push(format!("{version_folder_name}.jar"));
    File::create(&path_version).await?; // dummy jar required by the launcher.

    path_version.set_extension("json");
    let mut json_file = File::create(&path_version).await?;
    write_json::write_version(mc_version, quilt_version, &mut json_file).await?;

    mc_path.push("launcher_profiles");
    mc_path.set_extension("json");

    let mut launcher_file = File::open(&mc_path).await?;
    let profile = write_json::write_profile(mc_version, quilt_version, &mut launcher_file).await?;

    launcher_file = fs::File::create(&mc_path).await?;
    launcher_file.write_all(profile.as_bytes()).await?;

    Ok(path_mods)
}

fn get_minecraft_dir() -> Result<PathBuf> {
    cfg_select! {
        target_os = "linux" => {
            let mut dir = home_dir().ok_or_else(|| anyhow!("Couldn't find home directory!"))?;
            dir.push(".minecraft");
            Ok(dir)
        }
        target_os = "macos" => {
            let mut dir = home_dir().ok_or_else(|| anyhow!("Couldn't find home directory!"))?;
            dir.push("Library");
            dir.push("Application Support");
            dir.push("minecraft");
            Ok(dir)
        }
        target_os = "windows" => {
            let mut dir = home_dir().ok_or_else(|| anyhow!("Couldn't find home directory!"))?;
            dir.push("AppData");
            dir.push("Roaming");
            dir.push(".minecraft");
            Ok(dir)
        }
    }
}
