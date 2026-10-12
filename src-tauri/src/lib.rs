use tauri::Manager;
mod assistant;
mod nearby;
mod transfer;
mod updates;
#[tauri::command]
fn nearby_devices(service: tauri::State<'_, nearby::Nearby>) -> nearby::Snapshot {
    service.snapshot()
}
#[tauri::command]
fn set_nearby_enabled(
    app: tauri::AppHandle,
    service: tauri::State<'_, nearby::Nearby>,
    enabled: bool,
) -> Result<nearby::Snapshot, String> {
    let path = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("discovery-enabled.json");
    std::fs::write(path, if enabled { "true" } else { "false" }).map_err(|e| e.to_string())?;
    if !enabled {
        app.state::<transfer::Transfer>().reject();
    }
    Ok(service.set_enabled(enabled))
}
#[tauri::command]
async fn device_request(
    store: tauri::State<'_, garage_server::DeviceGarage>,
    path: String,
    token: String,
    data: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    Ok(store.call(&path, &token, data).await)
}
#[tauri::command]
async fn export_backup(app: tauri::AppHandle, contents: String) -> Result<String, String> {
    if contents.len() > 110_000_000 {
        return Err("Backup exceeds the storage limit".into());
    }
    let directory = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("exports");
    std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs();
    let path = directory.join(format!("project-garage-{timestamp}.json"));
    std::fs::write(&path, &contents).map_err(|e| e.to_string())?;
    #[cfg(target_os = "android")]
    {
        let shared = app
            .path()
            .app_cache_dir()
            .map_err(|e| e.to_string())?
            .join("project-garage-backup.json");
        std::fs::write(&shared, contents).map_err(|e| e.to_string())?;
        updates::share_agent_context(&app, &shared).await?;
    }
    Ok(path.to_string_lossy().into_owned())
}
#[tauri::command]
async fn export_agent_context(app: tauri::AppHandle, contents: String) -> Result<String, String> {
    if contents.len() > 1_000_000 {
        return Err("Agent context is too large".into());
    }
    let path = app
        .path()
        .app_cache_dir()
        .map_err(|e| e.to_string())?
        .join("project-garage-agent-context.json");
    std::fs::create_dir_all(path.parent().ok_or("Invalid cache path")?)
        .map_err(|e| e.to_string())?;
    std::fs::write(&path, contents).map_err(|e| e.to_string())?;
    updates::share_agent_context(&app, &path).await?;
    Ok(path.to_string_lossy().into_owned())
}

#[tauri::command]
fn transfer_status(service: tauri::State<'_, transfer::Transfer>) -> transfer::View {
    service.view()
}
#[tauri::command]
fn request_phone_import(
    service: tauri::State<'_, transfer::Transfer>,
    nearby: tauri::State<'_, nearby::Nearby>,
    id: String,
) -> Result<transfer::View, String> {
    let peer = nearby
        .snapshot()
        .peers
        .into_iter()
        .find(|p| p.id == id)
        .ok_or("Phone is no longer nearby")?;
    service.request(peer.address)
}
#[tauri::command]
async fn approve_phone_export(
    service: tauri::State<'_, transfer::Transfer>,
    id: String,
    contents: String,
) -> Result<transfer::View, String> {
    let service = service.inner().clone();
    tauri::async_runtime::spawn_blocking(move || service.approve(id, contents))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
fn reject_phone_export(service: tauri::State<'_, transfer::Transfer>) {
    service.reject()
}
#[tauri::command]
fn take_phone_import(service: tauri::State<'_, transfer::Transfer>) -> Result<String, String> {
    service.take()
}

#[tauri::command]
async fn export_spending_csv(app: tauri::AppHandle, contents: String) -> Result<String, String> {
    if contents.len() > 10_000_000 {
        return Err("Purchase export is too large".into());
    }
    let directory = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("exports");
    std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    let path = directory.join("project-garage-purchases.csv");
    std::fs::write(&path, &contents).map_err(|e| e.to_string())?;
    #[cfg(target_os = "android")]
    {
        let share = app
            .path()
            .app_cache_dir()
            .map_err(|e| e.to_string())?
            .join("project-garage-purchases.csv");
        std::fs::write(&share, contents).map_err(|e| e.to_string())?;
        updates::share_agent_context(&app, &share).await?;
    }
    Ok(path.to_string_lossy().into_owned())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(updates::plugin())
        .setup(|app| {
            let path = app.path().app_data_dir()?.join("garage.db");
            app.manage(garage_server::DeviceGarage::open(&path).map_err(std::io::Error::other)?);
            let preference = app.path().app_data_dir()?.join("discovery-enabled.json");
            let enabled = std::fs::read_to_string(preference)
                .ok()
                .and_then(|value| serde_json::from_str::<bool>(&value).ok())
                .unwrap_or(true);
            let transfer = transfer::Transfer::default();
            app.manage(nearby::Nearby::start(enabled, transfer.clone()));
            app.manage(transfer);
            app.manage(updates::Updates::new());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            assistant::ask_garage_ai,
            export_backup,
            export_spending_csv,
            export_agent_context,
            device_request,
            transfer_status,
            request_phone_import,
            approve_phone_export,
            reject_phone_export,
            take_phone_import,
            nearby_devices,
            set_nearby_enabled,
            updates::check_github_update,
            updates::install_github_update
        ])
        .run(tauri::generate_context!())
        .expect("Run Project Garage");
}
