use tauri::Manager;
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
fn export_backup(app: tauri::AppHandle, contents: String) -> Result<String, String> {
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
    std::fs::write(&path, contents).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().into_owned())
}
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let path = app.path().app_data_dir()?.join("garage.db");
            app.manage(garage_server::DeviceGarage::open(&path).map_err(std::io::Error::other)?);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![export_backup, device_request])
        .run(tauri::generate_context!())
        .expect("Run Project Garage");
}
