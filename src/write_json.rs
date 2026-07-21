use anyhow::Result;
use chrono::Utc;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use std::collections::HashMap;

const VANILLA_ARGS: &str = "-Xmx2G -XX:+UnlockExperimentalVMOptions -XX:+UseG1GC -XX:G1NewSizePercent=20 -XX:G1ReservePercent=20 -XX:MaxGCPauseMillis=50 -XX:G1HeapRegionSize=32M";
const ICON: &str = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAACAAAAAgBAMAAACBVGfHAAAAGFBMVEUAAAB7GWmyF1rqR1a3msT/nJfp0+r///+virVnAAAAAXRSTlMAQObYZgAAAI9JREFUKM990cENA0EIQ9FpgRbcglugBVr4Lbj9HCaRNtnZcHwSYMFaa63Vvb7rB5JS8QeiAqCfIA0C6yO/0CFJDQkcgWqSsTIlTpCSSJJoCAdAHqtqY/oOncgz492pIyTInrEK7kDtrci21AeQx+9olxwXWP0e2GRf4AaLRPYOV32CbSVb1TzAtsDloR94AcQTfFFwBa/NAAAAAElFTkSuQmCC";

#[derive(Serialize, Deserialize, Debug)]
struct LauncherProfiles {
    profiles: HashMap<String, Profile>,
    settings: Value,
    version: u8,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Profile {
    name: String,
    last_used: String,
    last_version_id: String,
    created: String,
    icon: String,
    #[serde(rename = "type")]
    _type: String,
    java_args: Option<String>,
    java_dir: Option<String>,
    #[serde(flatten)]
    extra: HashMap<String, Value>,
}

pub async fn write_version(
    client: Client,
    mc: &String,
    velvet: &String,
    z: &mut fs::File,
) -> Result<()> {
    let url = format!("https://meta.quiltmc.org/v3/versions/loader/{mc}/{velvet}/profile/json");
    let json: serde_json::Value = client
        .get(&url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    z.write_all(&serde_json::to_vec_pretty(&json)?).await?;
    Ok(())
}

pub async fn write_profile(mc: &String, velvet: &String, x: &mut fs::File) -> Result<String> {
    let mut bytes = Vec::new();
    x.read_to_end(&mut bytes).await?;
    let mut json: LauncherProfiles = serde_json::from_slice(&bytes)?;
    // This copies the "Latest Release" java arguments and version.
    let mut java_args = String::new();
    let mut java_dir: Option<String> = None;
    let mut extra = HashMap::new();

    for x in json.profiles.values() {
        if x._type == "latest-release" {
            java_args = x
                .java_args
                .as_ref()
                .map_or(VANILLA_ARGS.into(), |x| x.to_owned());
            java_dir = x.java_dir.as_ref().map(|x| x.to_owned());
            extra = x.extra.to_owned();
            break;
        }
    }
    java_args = format!(
        "-Dloader.modsDir=.velvet/mods/{mc} -Dloader.configDir=.velvet/config/{mc} {java_args}"
    );
    let time = Utc::now().to_string();
    let new_profile = Profile {
        created: time.clone(),
        icon: ICON.into(),
        java_args: Some(java_args),
        java_dir,
        last_used: time,
        last_version_id: format!("quilt-loader-{velvet}-{mc}"),
        name: format!("Velvet {mc}"),
        extra,
        _type: "custom".into(),
    };
    json.profiles
        .insert(format!("velvet-quilt-loader-{mc}"), new_profile);
    Ok(serde_json::to_string_pretty(&json)?)
}
