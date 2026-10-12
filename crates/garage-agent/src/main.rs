use serde_json::{json, Value};
use std::path::{Path, PathBuf};
fn default_database() -> Result<PathBuf, String> {
    #[cfg(target_os = "windows")]
    let base = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .ok_or("APPDATA is unavailable; use --db")?;
    #[cfg(target_os = "macos")]
    let base = PathBuf::from(std::env::var_os("HOME").ok_or("HOME is unavailable; use --db")?)
        .join("Library/Application Support");
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|p| PathBuf::from(p).join(".local/share")))
        .ok_or("Home directory is unavailable; use --db")?;
    Ok(base.join("com.projectgarage.mobile/garage.db"))
}
fn option(args: &[String], key: &str) -> Option<String> {
    args.windows(2).find(|a| a[0] == key).map(|a| a[1].clone())
}
fn context(mut snapshot: Value) -> Value {
    snapshot.as_object_mut().unwrap().remove("members");
    if let Some(user) = snapshot["user"].as_object_mut() {
        user.remove("email");
        user.remove("id");
    }
    for car in snapshot["garage"]["cars"].as_array_mut().unwrap() {
        car.as_object_mut().unwrap().remove("cover");
        for item in car["items"].as_array_mut().unwrap() {
            item.as_object_mut().unwrap().remove("receipt");
        }
        for entry in car["journal"].as_array_mut().unwrap() {
            entry.as_object_mut().unwrap().remove("photos");
            entry.as_object_mut().unwrap().remove("author");
        }
    }
    snapshot["protocol"] = json!("projectgarage-agent/1");
    snapshot
}
fn body(result: Value) -> Result<Value, String> {
    if result["status"].as_u64().unwrap_or(500) >= 400 {
        Err(result["body"]["error"]
            .as_str()
            .unwrap_or("Could not access garage")
            .into())
    } else {
        Ok(result["body"].clone())
    }
}
async fn execute(args: &[String]) -> Result<Value, String> {
    let command = args.first().map(String::as_str).unwrap_or("help");
    if command == "help" || command == "--help" {
        return Ok(
            json!({"protocol":"projectgarage-agent/1","commands":["snapshot [--db PATH] [--output FILE]","apply --file CHANGES.json [--db PATH] [--output FILE]","init --name NAME [--db PATH]"],"changes":{"revision":"latest snapshot revision","actions":"array of partial record actions; maximum 32"},"storage":"same desktop SQLite garage; no server or provider key required"}),
        );
    }
    if !["snapshot", "apply", "init"].contains(&command) {
        return Err("Use snapshot, apply, init, or help".into());
    }
    let mut index = 1;
    while index < args.len() {
        let flag = args[index].as_str();
        if !["--db", "--output"].contains(&flag)
            && !(command == "apply" && flag == "--file")
            && !(command == "init" && flag == "--name")
        {
            return Err(format!("Unsupported option: {flag}"));
        }
        if index + 1 >= args.len() || args[index + 1].starts_with("--") {
            return Err(format!("Missing value for {flag}"));
        }
        index += 2;
    }
    let path = option(args, "--db")
        .map(PathBuf::from)
        .map(Ok)
        .unwrap_or_else(default_database)?;
    if command == "init" && path.exists() {
        return Err("Garage already exists. Read its snapshot instead.".into());
    }
    if command != "init" && !path.is_file() {
        return Err("Desktop garage not found. Use --db for an existing desktop database, or init --name to create your personal garage. Android data needs a Settings file handoff.".into());
    }
    let initial_name = if command == "init" {
        Some(
            option(args, "--name")
                .filter(|n| !n.trim().is_empty() && n.len() <= 80)
                .ok_or("Use init --name with a name under 80 characters")?,
        )
    } else {
        None
    };
    let store = garage_server::DeviceGarage::open(&path)?;
    let snapshot = if command == "init" {
        let name = initial_name.unwrap();
        body(
            store
                .call("device/open", "", Some(json!({"name":name})))
                .await,
        )?["snapshot"]
            .clone()
    } else if command == "snapshot" {
        body(store.agent_call("garage", None).await)?
    } else {
        let file = option(args, "--file").ok_or("Use apply --file CHANGES.json")?;
        if std::fs::metadata(&file)
            .map_err(|_| "Cannot open changes file")?
            .len()
            > 1_000_000
        {
            return Err("Changes file exceeds 1 MB".into());
        }
        let bytes = std::fs::read(&file).map_err(|_| "Cannot read changes file")?;
        let changes: Value =
            serde_json::from_slice(bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(&bytes))
                .map_err(|_| "Changes file must be JSON")?;
        body(store.agent_call("agent/action", Some(changes)).await)?
    };
    Ok(context(snapshot))
}
#[tokio::main]
async fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    match execute(&args).await {
        Ok(value) => {
            let text = serde_json::to_string_pretty(&value).unwrap();
            if let Some(output) = option(&args, "--output") {
                if let Err(e) = std::fs::write(Path::new(&output), &text) {
                    eprintln!("{}", json!({"error":e.to_string()}));
                    std::process::exit(1);
                }
            } else {
                println!("{text}");
            }
        }
        Err(error) => {
            eprintln!("{}", json!({"error":error}));
            std::process::exit(1);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn init_read_patch_and_conflict_leave_records_and_media_intact() {
        let dir = std::env::temp_dir().join(format!("garage-agent-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let db = dir.join("garage.db");
        let d = db.to_string_lossy().to_string();
        let first = execute(&[
            "init".into(),
            "--name".into(),
            "Test".into(),
            "--db".into(),
            d.clone(),
        ])
        .await
        .unwrap();
        assert_eq!(first["revision"], 0);
        let changes = dir.join("changes.json");
        std::fs::write(&changes,json!({"protocol":"projectgarage-agent/1","revision":0,"actions":[{"type":"save_car","car":{"id":"","name":"E30","purchase_price_cents":100000}}]}).to_string()).unwrap();
        let args = vec![
            "apply".into(),
            "--db".into(),
            d.clone(),
            "--file".into(),
            changes.to_string_lossy().into_owned(),
        ];
        let saved = execute(&args).await.unwrap();
        assert_eq!(saved["garage"]["cars"][0]["name"], "E30");
        assert_eq!(saved["revision"], 1);
        assert!(execute(&args).await.is_err());
        let read = execute(&["snapshot".into(), "--db".into(), d])
            .await
            .unwrap();
        assert_eq!(read["revision"], 1);
        assert!(read["garage"]["cars"][0].get("cover").is_none());
        assert!(read["user"].get("email").is_none());
        for entry in std::fs::read_dir(&dir).unwrap() {
            std::fs::remove_file(entry.unwrap().path()).unwrap();
        }
        std::fs::remove_dir(dir).unwrap();
    }
}
