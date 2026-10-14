use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, sync::Mutex, time::Duration};
use tauri::Manager;
const BASE: &str = "https://raw.githubusercontent.com/dustindwayne/projectgarage/main";
const LIMIT: u64 = 100_000_000;
#[derive(Clone, Deserialize, Serialize)]
pub struct Package {
    pub path: String,
    pub sha256: String,
    pub size: u64,
}
#[derive(Clone, Deserialize, Serialize)]
pub struct Manifest {
    pub version: String,
    pub notes: String,
    pub platforms: HashMap<String, Package>,
}
#[derive(Clone, Serialize)]
pub struct UpdateInfo {
    pub current_version: String,
    pub latest_version: String,
    pub available: bool,
    pub notes: String,
    pub platform: String,
    pub package_available: bool,
}
pub struct Updates {
    client: reqwest::Client,
    manifest: Mutex<Option<Manifest>>,
    busy: std::sync::atomic::AtomicBool,
}
impl Updates {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .user_agent("Project-Garage-Updater")
                .timeout(Duration::from_secs(180))
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .expect("Build HTTPS client"),
            manifest: Mutex::new(None),
            busy: std::sync::atomic::AtomicBool::new(false),
        }
    }
    async fn request(&self, path: &str) -> Result<reqwest::Response, String> {
        let request = self.client.get(format!("{BASE}/{path}"));
        let response = request
            .send()
            .await
            .map_err(|_| "Could not reach GitHub. Check your internet connection.".to_string())?;
        match response.status().as_u16() {
            200 => Ok(response),
            404 => Err("The update channel is not available yet. Try again later.".into()),
            401 | 403 => Err(
                "The public download channel is temporarily unavailable. Try again later.".into(),
            ),
            _ => Err(format!(
                "GitHub could not supply the update ({}). Try again later.",
                response.status()
            )),
        }
    }
    pub async fn check(&self) -> Result<UpdateInfo, String> {
        let response = self.request("downloads/latest.json").await?;
        if response.content_length().is_some_and(|n| n > 100_000) {
            return Err("Invalid update manifest".into());
        }
        let bytes = response
            .bytes()
            .await
            .map_err(|_| "Could not read the update information")?;
        if bytes.len() > 100_000 {
            return Err("Invalid update manifest".into());
        }
        let manifest: Manifest =
            serde_json::from_slice(&bytes).map_err(|_| "Invalid update manifest")?;
        let available = newer(&manifest.version, env!("CARGO_PKG_VERSION"))?;
        let key = platform();
        if let Some(package) = manifest.platforms.get(&key) {
            validate_package(package, &key)?;
        }
        let info = UpdateInfo {
            current_version: env!("CARGO_PKG_VERSION").into(),
            latest_version: manifest.version.clone(),
            available,
            notes: manifest.notes.clone(),
            platform: key.clone(),
            package_available: manifest.platforms.contains_key(&key),
        };
        *self.manifest.lock().unwrap() = Some(manifest);
        Ok(info)
    }
    async fn download(&self, app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
        let manifest = self
            .manifest
            .lock()
            .unwrap()
            .clone()
            .ok_or("Check for an update first")?;
        if !newer(&manifest.version, env!("CARGO_PKG_VERSION"))? {
            return Err("You already have this version".into());
        }
        let key = platform();
        let package = manifest
            .platforms
            .get(&key)
            .ok_or("No installer is published for this platform yet")?;
        validate_package(package, &key)?;
        let response = self.request(&package.path).await?;
        if response.content_length().is_some_and(|n| n != package.size) {
            return Err("Update download size did not match its manifest".into());
        }
        let directory = app
            .path()
            .app_cache_dir()
            .map_err(|e| e.to_string())?
            .join("updates");
        std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
        let extension = if cfg!(target_os = "android") {
            "apk"
        } else {
            "exe"
        };
        let pending = directory.join(format!("project-garage-update.{extension}.part"));
        let target = directory.join(format!("project-garage-update.{extension}"));
        let result: Result<(), String> = async {
            use std::io::Write;
            let mut file = std::fs::File::create(&pending).map_err(|e| e.to_string())?;
            let mut total = 0;
            let mut hash = Sha256::new();
            let mut response = response;
            while let Some(chunk) = response
                .chunk()
                .await
                .map_err(|_| "The download was interrupted. Try again.")?
            {
                total += chunk.len() as u64;
                if total > package.size || total > LIMIT {
                    return Err("Update exceeded the expected download size".into());
                }
                hash.update(&chunk);
                file.write_all(&chunk).map_err(|e| e.to_string())?;
            }
            verify_digest(total, &format!("{:x}", hash.finalize()), package)?;
            file.sync_all().map_err(|e| e.to_string())?;
            drop(file);
            if target.exists() {
                std::fs::remove_file(&target).map_err(|e| e.to_string())?;
            }
            std::fs::rename(&pending, &target).map_err(|e| e.to_string())?;
            Ok(())
        }
        .await;
        if result.is_err() {
            let _ = std::fs::remove_file(&pending);
        }
        result?;
        Ok(target)
    }
}
pub fn platform() -> String {
    format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
}
fn newer(latest: &str, current: &str) -> Result<bool, String> {
    let latest = semver::Version::parse(latest).map_err(|_| "Invalid update version")?;
    let current = semver::Version::parse(current).map_err(|_| "Invalid app version")?;
    Ok(latest > current)
}
fn validate_package(package: &Package, key: &str) -> Result<(), String> {
    let path = if key.starts_with("android-") {
        "downloads/project-garage-arm64-debug.apk"
    } else if key == "windows-x86_64" {
        "downloads/project-garage-windows.exe"
    } else {
        return Err("No installer is available for this platform".into());
    };
    if package.path != path
        || package.size == 0
        || package.size > LIMIT
        || package.sha256.len() != 64
        || !package.sha256.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return Err("Invalid update package".into());
    }
    Ok(())
}
fn verify_digest(size: u64, digest: &str, package: &Package) -> Result<(), String> {
    if size != package.size || digest != package.sha256.to_lowercase() {
        Err("Update verification failed. Nothing was installed.".into())
    } else {
        Ok(())
    }
}
#[tauri::command]
pub async fn check_github_update(updates: tauri::State<'_, Updates>) -> Result<UpdateInfo, String> {
    updates.check().await
}
#[tauri::command]
pub async fn install_github_update(
    app: tauri::AppHandle,
    updates: tauri::State<'_, Updates>,
) -> Result<String, String> {
    if updates.busy.swap(true, std::sync::atomic::Ordering::SeqCst) {
        return Err("An update is already downloading".into());
    }
    let result = async {
        let path = updates.download(&app).await?;
        #[cfg(target_os = "android")]
        {
            let response: serde_json::Value = app
                .state::<AndroidInstaller>()
                .0
                .run_mobile_plugin(
                    "install",
                    serde_json::json!({"path":path.to_string_lossy()}),
                )
                .map_err(|e| e.to_string())?;
            return Ok(response["status"]
                .as_str()
                .unwrap_or("installer_opened")
                .into());
        }
        #[cfg(target_os = "windows")]
        {
            windows_install(&app, &path)?;
            return Ok("restarting".into());
        }
        #[cfg(not(any(target_os = "android", target_os = "windows")))]
        {
            let _ = path;
            Err("Installation is not available for this platform yet".into())
        }
    }
    .await;
    updates
        .busy
        .store(false, std::sync::atomic::Ordering::SeqCst);
    result
}
#[cfg(target_os = "android")]
struct AndroidInstaller(tauri::plugin::PluginHandle<tauri::Wry>);
pub fn plugin() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    tauri::plugin::Builder::new("garage-updates")
        .setup(|app, api| {
            #[cfg(target_os = "android")]
            app.manage(AndroidInstaller(api.register_android_plugin(
                "com.projectgarage.mobile",
                "GarageUpdatesPlugin",
            )?));
            #[cfg(not(target_os = "android"))]
            let _ = (app, api);
            Ok(())
        })
        .build()
}
#[cfg(target_os = "windows")]
fn windows_install(app: &tauri::AppHandle, download: &std::path::Path) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    let target = std::env::current_exe().map_err(|e| e.to_string())?;
    let probe = target
        .parent()
        .ok_or("Could not locate the installed app")?
        .join(format!(
            ".projectgarage-update-check-{}",
            uuid::Uuid::new_v4()
        ));
    std::fs::write(&probe,[]).map_err(|_|"This app location is not writable. Download the Windows update from GitHub and replace it with administrator approval.".to_string())?;
    let _ = std::fs::remove_file(probe);
    let directory = download.parent().ok_or("Invalid update directory")?;
    let script = directory.join("install-update.ps1");
    let result = directory.join("install-result.txt");
    let content = r#"param([string]$Target,[string]$Download,[int]$AppPid,[string]$Result)
$ErrorActionPreference='Stop'
try {
 $process=Get-Process -Id $AppPid -ErrorAction SilentlyContinue
 if($process){$process.WaitForExit(30000)|Out-Null}
 for($attempt=0;$attempt -lt 20;$attempt++){try{Copy-Item -LiteralPath $Download -Destination $Target -Force;break}catch{if($attempt -eq 19){throw};Start-Sleep -Milliseconds 500}}
 Set-Content -LiteralPath $Result -Value 'Update installed'
 Start-Process -FilePath $Target -WindowStyle Normal
} catch {Set-Content -LiteralPath $Result -Value ('Update failed: '+$_.Exception.Message)}
"#;
    std::fs::write(&script, content).map_err(|e| e.to_string())?;
    std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
        .arg(&script)
        .arg("-Target")
        .arg(&target)
        .arg("-Download")
        .arg(download)
        .arg("-AppPid")
        .arg(std::process::id().to_string())
        .arg("-Result")
        .arg(result)
        .creation_flags(0x08000000)
        .spawn()
        .map_err(|_| {
            "Could not start the updater. Download the Windows app from GitHub manually."
                .to_string()
        })?;
    app.exit(0);
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_newer_versions_update() {
        assert!(newer("0.2.0", "0.1.0").unwrap());
        assert!(!newer("0.2.0", "0.2.0").unwrap());
        assert!(!newer("0.1.0", "0.2.0").unwrap());
        assert!(newer("bad", "0.2.0").is_err());
    }
    #[test]
    fn rejects_untrusted_paths_and_invalid_hashes() {
        let mut package = Package {
            path: "downloads/project-garage-arm64-debug.apk".into(),
            sha256: "a".repeat(64),
            size: 24000000,
        };
        assert!(validate_package(&package, "android-aarch64").is_ok());
        package.path = "https://other.example/app.apk".into();
        assert!(validate_package(&package, "android-aarch64").is_err());
        package.path = "downloads/project-garage-arm64-debug.apk".into();
        package.sha256 = "bad".into();
        assert!(validate_package(&package, "android-aarch64").is_err());
    }
    #[test]
    fn modified_or_truncated_downloads_are_rejected() {
        let package = Package {
            path: "downloads/project-garage-arm64-debug.apk".into(),
            size: 3,
            sha256: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad".into(),
        };
        assert!(verify_digest(3, &format!("{:x}", Sha256::digest(b"abc")), &package).is_ok());
        assert!(verify_digest(3, &format!("{:x}", Sha256::digest(b"abd")), &package).is_err());
        assert!(verify_digest(2, &package.sha256, &package).is_err());
    }
    #[test]
    #[ignore = "Requires live access to the public update channel"]
    fn github_channel_and_binary_download_match() {
        tauri::async_runtime::block_on(async {
            let updates = Updates::new();
            let info = updates.check().await.unwrap();
            assert!(info.package_available);
            let manifest = updates.manifest.lock().unwrap().clone().unwrap();
            let package = manifest.platforms.get(&platform()).unwrap();
            let response = updates.request(&package.path).await.unwrap();
            let bytes = response.bytes().await.unwrap();
            verify_digest(
                bytes.len() as u64,
                &format!("{:x}", Sha256::digest(&bytes)),
                package,
            )
            .unwrap();
        });
    }
}

pub async fn share_agent_context(
    app: &tauri::AppHandle,
    path: &std::path::Path,
) -> Result<(), String> {
    #[cfg(target_os = "android")]
    app.state::<AndroidInstaller>()
        .0
        .run_mobile_plugin::<serde_json::Value>(
            "shareContext",
            serde_json::json!({"path":path.to_string_lossy()}),
        )
        .map_err(|e| e.to_string())?;
    #[cfg(not(target_os = "android"))]
    let _ = (app, path);
    Ok(())
}
