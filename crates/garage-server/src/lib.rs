use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
};
use axum::{
    extract::{DefaultBodyLimit, State},
    http::{HeaderMap, Method, StatusCode},
    routing::{get, post},
    Json, Router,
};
use garage_core::{new_id, Action, Garage, Role};
use rusqlite::{params, Connection, OptionalExtension};
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};
use tower_http::{
    cors::CorsLayer,
    services::{ServeDir, ServeFile},
};

type Db = Arc<Mutex<Connection>>;
type ApiResult = Result<Json<Value>, ApiError>;
struct ApiError(StatusCode, String);
impl axum::response::IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        (self.0, Json(json!({"error":self.1}))).into_response()
    }
}
impl From<rusqlite::Error> for ApiError {
    fn from(e: rusqlite::Error) -> Self {
        eprintln!("Database error: {e}");
        Self(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Could not save garage data".into(),
        )
    }
}
fn bad(s: impl Into<String>) -> ApiError {
    ApiError(StatusCode::BAD_REQUEST, s.into())
}
fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}
fn digest(s: &str) -> String {
    format!("{:x}", Sha256::digest(s.as_bytes()))
}
#[derive(Clone)]
struct App {
    db: Db,
    attempts: Arc<Mutex<HashMap<String, (i64, u32)>>>,
}
fn database(path: &str) -> Connection {
    let db = Connection::open(path).expect("Open garage database");
    db.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;
      CREATE TABLE IF NOT EXISTS garages(id TEXT PRIMARY KEY, data TEXT NOT NULL, revision INTEGER NOT NULL DEFAULT 0);
      CREATE TABLE IF NOT EXISTS users(id TEXT PRIMARY KEY, email TEXT NOT NULL UNIQUE, name TEXT NOT NULL, password TEXT NOT NULL, garage_id TEXT NOT NULL REFERENCES garages(id), role TEXT NOT NULL);
      CREATE TABLE IF NOT EXISTS sessions(token TEXT PRIMARY KEY, user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE, expires INTEGER NOT NULL);
      CREATE TABLE IF NOT EXISTS invites(token TEXT PRIMARY KEY, garage_id TEXT NOT NULL REFERENCES garages(id), expires INTEGER NOT NULL);").expect("Initialize database");
    db
}
struct Actor {
    id: String,
    name: String,
    email: String,
    garage_id: String,
    role: Role,
}
fn actor(db: &Connection, headers: &HeaderMap) -> Result<Actor, ApiError> {
    let token = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(ApiError(
            StatusCode::UNAUTHORIZED,
            "Sign in to continue".into(),
        ))?;
    db.query_row("SELECT u.id,u.name,u.email,u.garage_id,u.role FROM users u JOIN sessions s ON s.user_id=u.id WHERE s.token=? AND s.expires>?",params![digest(token),now()],|r|Ok(Actor{id:r.get(0)?,name:r.get(1)?,email:r.get(2)?,garage_id:r.get(3)?,role:if r.get::<_,String>(4)?=="owner"{Role::Owner}else{Role::Staff}})).optional()?.ok_or(ApiError(StatusCode::UNAUTHORIZED,"Your session expired. Sign in again.".into()))
}
fn snapshot(db: &Connection, a: &Actor) -> Result<Value, ApiError> {
    let (data, revision): (String, i64) = db.query_row(
        "SELECT data,revision FROM garages WHERE id=?",
        [&a.garage_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    let garage: Value = serde_json::from_str(&data).map_err(|_| bad("Garage data is invalid"))?;
    let mut stmt =
        db.prepare("SELECT id,name,email,role FROM users WHERE garage_id=? ORDER BY role,name")?;
    let members=stmt.query_map([&a.garage_id],|r|Ok(json!({"id":r.get::<_,String>(0)?,"name":r.get::<_,String>(1)?,"email":r.get::<_,String>(2)?,"role":r.get::<_,String>(3)?})))?.collect::<Result<Vec<_>,_>>()?;
    Ok(
        json!({"garage":garage,"revision":revision,"user":{"id":a.id,"name":a.name,"email":a.email,"role":a.role},"members":members}),
    )
}
#[derive(Deserialize)]
struct AuthInput {
    email: String,
    password: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    invite: String,
}
fn auth_limit(app: &App, email: &str) -> Result<(), ApiError> {
    let mut attempts = app.attempts.lock().unwrap();
    attempts.retain(|_, v| now() - v.0 < 300);
    let e = attempts.entry(email.to_lowercase()).or_insert((now(), 0));
    e.1 += 1;
    if e.1 > 15 {
        return Err(ApiError(
            StatusCode::TOO_MANY_REQUESTS,
            "Too many attempts. Try again in five minutes.".into(),
        ));
    }
    Ok(())
}
async fn register(State(app): State<App>, Json(input): Json<AuthInput>) -> ApiResult {
    auth_limit(&app, &input.email)?;
    let email = input.email.trim().to_lowercase();
    let name = input.name.trim();
    if name.is_empty()
        || name.len() > 80
        || !email.contains('@')
        || email.len() > 254
        || input.password.len() < 10
        || input.password.len() > 256
    {
        return Err(bad(
            "Enter your name, email, and a password of 10–256 characters",
        ));
    }
    let hash = Argon2::default()
        .hash_password(input.password.as_bytes(), &SaltString::generate(&mut OsRng))
        .map_err(|_| bad("Could not create account"))?
        .to_string();
    let mut db = app.db.lock().unwrap();
    let tx = db.transaction()?;
    if tx
        .query_row("SELECT id FROM users WHERE email=?", [&email], |r| {
            r.get::<_, String>(0)
        })
        .optional()?
        .is_some()
    {
        return Err(bad("That email already has an account. Sign in instead."));
    }
    let gid;
    let role;
    if input.invite.trim().is_empty() {
        gid = new_id();
        role = "owner";
        tx.execute(
            "INSERT INTO garages(id,data) VALUES(?,?)",
            params![gid, serde_json::to_string(&Garage::new(name)).unwrap()],
        )?;
    } else {
        gid = tx
            .query_row(
                "SELECT garage_id FROM invites WHERE token=? AND expires>?",
                params![digest(input.invite.trim()), now()],
                |r| r.get::<_, String>(0),
            )
            .optional()?
            .ok_or(bad("This invitation is expired or already used"))?;
        role = "staff";
        tx.execute(
            "DELETE FROM invites WHERE token=?",
            [digest(input.invite.trim())],
        )?;
    }
    let uid = new_id();
    tx.execute(
        "INSERT INTO users VALUES(?,?,?,?,?,?)",
        params![uid, email, name, hash, gid, role],
    )?;
    let token = format!("{}{}", new_id(), new_id());
    tx.execute(
        "INSERT INTO sessions VALUES(?,?,?)",
        params![digest(&token), uid, now() + 30 * 86400],
    )?;
    tx.commit()?;
    let a = Actor {
        id: uid,
        name: name.into(),
        email,
        garage_id: gid,
        role: if role == "owner" {
            Role::Owner
        } else {
            Role::Staff
        },
    };
    Ok(Json(json!({"token":token,"snapshot":snapshot(&db,&a)?})))
}
async fn login(State(app): State<App>, Json(input): Json<AuthInput>) -> ApiResult {
    auth_limit(&app, &input.email)?;
    if input.password.len() > 256 {
        return Err(bad("Invalid credentials"));
    }
    let db = app.db.lock().unwrap();
    let row = db
        .query_row(
            "SELECT id,password FROM users WHERE email=?",
            [input.email.trim().to_lowercase()],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
        )
        .optional()?;
    let (id, hash) = row.ok_or(ApiError(
        StatusCode::UNAUTHORIZED,
        "Email or password is incorrect".into(),
    ))?;
    if Argon2::default()
        .verify_password(
            input.password.as_bytes(),
            &PasswordHash::new(&hash).map_err(|_| bad("Account password is invalid"))?,
        )
        .is_err()
    {
        return Err(ApiError(
            StatusCode::UNAUTHORIZED,
            "Email or password is incorrect".into(),
        ));
    }
    let token = format!("{}{}", new_id(), new_id());
    db.execute("DELETE FROM sessions WHERE expires<?", [now()])?;
    db.execute(
        "INSERT INTO sessions VALUES(?,?,?)",
        params![digest(&token), id, now() + 30 * 86400],
    )?;
    let mut headers = HeaderMap::new();
    headers.insert("authorization", format!("Bearer {token}").parse().unwrap());
    Ok(Json(
        json!({"token":token,"snapshot":snapshot(&db,&actor(&db,&headers)?)?}),
    ))
}
async fn read(State(app): State<App>, headers: HeaderMap) -> ApiResult {
    let db = app.db.lock().unwrap();
    Ok(Json(snapshot(&db, &actor(&db, &headers)?)?))
}
#[derive(Deserialize)]
struct AgentMutation {
    protocol: String,
    revision: i64,
    actions: Value,
}
async fn agent_mutate(
    State(app): State<App>,
    headers: HeaderMap,
    Json(input): Json<AgentMutation>,
) -> ApiResult {
    if input.protocol != "projectgarage-agent/1" {
        return Err(bad("Unsupported agent changes protocol"));
    }
    let current = read(State(app.clone()), headers.clone()).await?.0;
    let actions = garage_core::agent::normalize(&input.actions, &current).map_err(bad)?;
    let action = serde_json::from_value(json!({"type":"batch","actions":actions}))
        .map_err(|_| bad("Invalid agent action"))?;
    mutate(
        State(app),
        headers,
        Json(Mutation {
            revision: input.revision,
            action,
        }),
    )
    .await
}
#[derive(Deserialize)]
struct Mutation {
    revision: i64,
    action: Action,
}
async fn mutate(
    State(app): State<App>,
    headers: HeaderMap,
    Json(input): Json<Mutation>,
) -> ApiResult {
    let mut db = app.db.lock().unwrap();
    let a = actor(&db, &headers)?;
    let tx = db.transaction()?;
    let (data, revision): (String, i64) = tx.query_row(
        "SELECT data,revision FROM garages WHERE id=?",
        [&a.garage_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    if revision != input.revision {
        return Err(ApiError(StatusCode::CONFLICT,"Someone updated the garage. Your view has been refreshed; review your change and save again.".into()));
    }
    let mut garage: Garage =
        serde_json::from_str(&data).map_err(|_| bad("Garage data is invalid"))?;
    garage
        .apply(&a.role, &a.email, input.action)
        .map_err(|e| ApiError(StatusCode::FORBIDDEN, e))?;
    let data = serde_json::to_string(&garage).unwrap();
    if data.len() > 100_000_000 {
        return Err(bad(
            "Garage has reached the 100 MB storage limit. Export a backup and reduce photo size.",
        ));
    }
    tx.execute(
        "UPDATE garages SET data=?,revision=revision+1 WHERE id=?",
        params![data, a.garage_id],
    )?;
    tx.commit()?;
    Ok(Json(snapshot(&db, &a)?))
}
async fn invite(State(app): State<App>, headers: HeaderMap) -> ApiResult {
    let db = app.db.lock().unwrap();
    let a = actor(&db, &headers)?;
    if a.role != Role::Owner {
        return Err(ApiError(
            StatusCode::FORBIDDEN,
            "Only the owner can invite staff".into(),
        ));
    }
    let token = new_id();
    db.execute("DELETE FROM invites WHERE expires<?", [now()])?;
    db.execute(
        "INSERT INTO invites VALUES(?,?,?)",
        params![digest(&token), a.garage_id, now() + 7 * 86400],
    )?;
    Ok(Json(json!({"code":token,"expires":now()+7*86400})))
}
#[derive(Deserialize)]
struct RemoveMember {
    id: String,
}
async fn remove_member(
    State(app): State<App>,
    headers: HeaderMap,
    Json(input): Json<RemoveMember>,
) -> ApiResult {
    let db = app.db.lock().unwrap();
    let a = actor(&db, &headers)?;
    if a.role != Role::Owner {
        return Err(ApiError(
            StatusCode::FORBIDDEN,
            "Only the owner can remove staff".into(),
        ));
    }
    db.execute(
        "DELETE FROM users WHERE id=? AND garage_id=? AND role='staff'",
        params![input.id, a.garage_id],
    )?;
    Ok(Json(snapshot(&db, &a)?))
}
async fn logout(State(app): State<App>, headers: HeaderMap) -> ApiResult {
    let db = app.db.lock().unwrap();
    if let Some(token) = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
    {
        db.execute("DELETE FROM sessions WHERE token=?", [digest(token)])?;
    }
    Ok(Json(json!({"ok":true})))
}
fn router(app: App) -> Router {
    let origins=std::env::var("GARAGE_ORIGINS").unwrap_or("http://localhost:1420,http://127.0.0.1:1420,http://tauri.localhost,https://tauri.localhost,tauri://localhost".into()).split(',').filter_map(|s|s.trim().parse().ok()).collect::<Vec<_>>();
    Router::new()
        .route("/api/health", get(|| async { Json(json!({"ok":true})) }))
        .route("/api/register", post(register))
        .route("/api/login", post(login))
        .route("/api/logout", post(logout))
        .route("/api/garage", get(read))
        .route("/api/action", post(mutate))
        .route("/api/agent/action", post(agent_mutate))
        .route("/api/invite", post(invite))
        .route("/api/member/remove", post(remove_member))
        .layer(DefaultBodyLimit::max(24 * 1024 * 1024))
        .layer(
            CorsLayer::new()
                .allow_origin(origins)
                .allow_methods([Method::GET, Method::POST])
                .allow_headers([
                    axum::http::header::AUTHORIZATION,
                    axum::http::header::CONTENT_TYPE,
                ]),
        )
        .with_state(app)
        .fallback_service(
            ServeDir::new("dist").not_found_service(ServeFile::new("dist/index.html")),
        )
}
pub async fn run_server() {
    let path = std::env::var("GARAGE_DB").unwrap_or("data/garage.db".into());
    if let Some(parent) = std::path::Path::new(&path).parent() {
        std::fs::create_dir_all(parent).expect("Create data directory");
    }
    let app = App {
        db: Arc::new(Mutex::new(database(&path))),
        attempts: Default::default(),
    };
    let address = std::env::var("GARAGE_BIND").unwrap_or("127.0.0.1:3030".into());
    let listener = tokio::net::TcpListener::bind(&address)
        .await
        .expect("Bind server");
    println!("Project Garage is open at http://{address}");
    axum::serve(listener, router(app)).await.unwrap();
}
/// On-device storage. Calls garage rules directly: no listener, sockets, or service process.
pub struct DeviceGarage {
    app: App,
}
impl DeviceGarage {
    pub fn open(path: &std::path::Path) -> Result<Self, String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let db = database(&path.to_string_lossy());
        db.execute_batch("CREATE TABLE IF NOT EXISTS device_profile(user_id TEXT PRIMARY KEY REFERENCES users(id));").map_err(|e| e.to_string())?;
        Ok(Self {
            app: App {
                db: Arc::new(Mutex::new(db)),
                attempts: Default::default(),
            },
        })
    }
    fn open_profile(&self, input: Option<Value>, rotate: bool) -> ApiResult {
        let mut db = self.app.db.lock().unwrap();
        let tx = db.transaction()?;
        let existing: Option<String> = tx
            .query_row("SELECT user_id FROM device_profile LIMIT 1", [], |r| {
                r.get(0)
            })
            .optional()?;
        let uid = if let Some(uid) = existing {
            uid
        } else {
            let name = input
                .as_ref()
                .and_then(|v| v.get("name"))
                .and_then(Value::as_str)
                .unwrap_or("")
                .trim();
            if name.is_empty() {
                return Ok(Json(json!({"needs_name":true})));
            }
            if name.len() > 80 {
                return Err(bad("Use a name under 80 characters"));
            }
            let gid = new_id();
            let uid = new_id();
            tx.execute(
                "INSERT INTO garages(id,data) VALUES(?,?)",
                params![gid, serde_json::to_string(&Garage::new(name)).unwrap()],
            )?;
            tx.execute(
                "INSERT INTO users VALUES(?,?,?,?,?,?)",
                params![uid, format!("{uid}@device.invalid"), name, "", gid, "owner"],
            )?;
            tx.execute("INSERT INTO device_profile VALUES(?)", [&uid])?;
            uid
        };
        if rotate {
            tx.execute("DELETE FROM sessions WHERE user_id=?", [&uid])?;
        }
        let token = format!("{}{}", new_id(), new_id());
        tx.execute(
            "INSERT INTO sessions VALUES(?,?,?)",
            params![digest(&token), uid, now() + 30 * 86400],
        )?;
        tx.commit()?;
        let mut headers = HeaderMap::new();
        headers.insert("authorization", format!("Bearer {token}").parse().unwrap());
        Ok(Json(
            json!({"token":token,"snapshot":snapshot(&db,&actor(&db,&headers)?)?}),
        ))
    }
    /// A short-lived external-agent session never invalidates the running app's session.
    pub async fn agent_call(&self, path: &str, data: Option<Value>) -> Value {
        let opened = match self.open_profile(None, false) {
            Ok(Json(value)) => value,
            Err(ApiError(status, error)) => {
                return json!({"status":status.as_u16(),"body":{"error":error}})
            }
        };
        let Some(token) = opened["token"].as_str() else {
            return json!({"status":400,"body":{"error":"Open Project Garage and enter your name first"}});
        };
        let result = self.call(path, token, data).await;
        let _ = self.call("logout", token, Some(json!({}))).await;
        result
    }
    pub async fn call(&self, path: &str, token: &str, data: Option<Value>) -> Value {
        let result: ApiResult = async {
            if path == "device/open" {
                return self.open_profile(data, true);
            }
            let mut headers = HeaderMap::new();
            headers.insert(
                "authorization",
                format!("Bearer {token}")
                    .parse()
                    .map_err(|_| bad("Invalid local session"))?,
            );
            match path {
                "garage" => read(State(self.app.clone()), headers).await,
                "action" => {
                    let input = serde_json::from_value(data.ok_or(bad("Missing change"))?)
                        .map_err(|_| bad("Invalid change"))?;
                    mutate(State(self.app.clone()), headers, Json(input)).await
                }
                "agent/action" => {
                    let input = serde_json::from_value(data.ok_or(bad("Missing agent changes"))?)
                        .map_err(|_| bad("Invalid agent changes"))?;
                    agent_mutate(State(self.app.clone()), headers, Json(input)).await
                }
                "logout" => logout(State(self.app.clone()), headers).await,
                "health" => Ok(Json(json!({"ok":true,"storage":"device"}))),
                _ => Err(bad(
                    "This action requires an optional shared garage connection",
                )),
            }
        }
        .await;
        match result {
            Ok(Json(body)) => json!({"status":200,"body":body}),
            Err(ApiError(status, error)) => {
                json!({"status":status.as_u16(),"body":{"error":error}})
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request};
    use http_body_util::BodyExt;
    use tower::ServiceExt;
    async fn request(
        app: Router,
        path: &str,
        token: &str,
        data: Option<Value>,
    ) -> (StatusCode, Value) {
        let mut req = Request::builder()
            .uri(path)
            .header("authorization", format!("Bearer {token}"));
        let body = if let Some(data) = data {
            req = req
                .method("POST")
                .header("content-type", "application/json");
            Body::from(data.to_string())
        } else {
            Body::empty()
        };
        let res = app.oneshot(req.body(body).unwrap()).await.unwrap();
        let status = res.status();
        let body = res.into_body().collect().await.unwrap().to_bytes();
        (status, serde_json::from_slice(&body).unwrap())
    }
    #[tokio::test]
    async fn agent_sessions_preserve_app_access_and_batches_reject_stale_or_invalid_changes() {
        let path = std::env::temp_dir().join(format!("garage-agent-session-{}.db", new_id()));
        let store = DeviceGarage::open(&path).unwrap();
        let opened = store
            .call("device/open", "", Some(json!({"name":"Test"})))
            .await;
        let token = opened["body"]["token"].as_str().unwrap();
        assert_eq!(store.agent_call("garage", None).await["status"], 200);
        assert_eq!(store.call("garage", token, None).await["status"], 200);
        let change = json!({"protocol":"projectgarage-agent/1","revision":0,"actions":[{"type":"configure","name":"Agent updated","currency":"USD"}]});
        assert_eq!(
            store.agent_call("agent/action", Some(change.clone())).await["status"],
            200
        );
        assert_eq!(
            store.agent_call("agent/action", Some(change)).await["status"],
            409
        );
        let invalid = json!({"protocol":"projectgarage-agent/1","revision":1,"actions":[{"type":"configure","name":"Must roll back","currency":"USD"},{"type":"delete_car","car_id":"missing"}]});
        assert_eq!(
            store.agent_call("agent/action", Some(invalid)).await["status"],
            403
        );
        let read = store.call("garage", token, None).await;
        assert_eq!(read["body"]["garage"]["name"], "Agent updated");
        assert_eq!(read["body"]["revision"], 1);
        drop(store);
        let _ = std::fs::remove_file(path);
    }
    #[tokio::test]
    async fn device_garage_persists_without_a_server_or_account() {
        let directory = std::env::temp_dir().join(format!("projectgarage-test-{}", new_id()));
        let path = directory.join("garage.db");
        {
            let store = DeviceGarage::open(&path).unwrap();
            let first = store.call("device/open", "", None).await;
            assert_eq!(first["body"]["needs_name"], true);
            let opened = store
                .call("device/open", "", Some(json!({"name":"Kevin"})))
                .await;
            let token = opened["body"]["token"].as_str().unwrap();
            assert_eq!(
                opened["body"]["snapshot"]["garage"]["name"],
                "Kevin's Project Garage"
            );
            let car = json!({"id":"","name":"Weekend build","year":"1989","make":"BMW","model":"325i","color":"#72806a","notes":"","cover":"","items":[],"journal":[]});
            let saved = store
                .call(
                    "action",
                    token,
                    Some(json!({"revision":0,"action":{"type":"save_car","car":car}})),
                )
                .await;
            assert_eq!(saved["status"], 200);
            let cid = saved["body"]["garage"]["cars"][0]["id"].as_str().unwrap();
            let part = json!({"id":"","name":"Brakes","kind":"part","status":"purchased","required":true,"pending":false,"price_cents":24995,"seller":"Parts store","purchased_on":"2026-10-12","warranty":"12 months","receipt":"data:image/png;base64,aGVsbG8=","receipt_name":"receipt.png","notes":""});
            let saved=store.call("action",token,Some(json!({"revision":1,"action":{"type":"save_item","car_id":cid,"item":part}}))).await;
            assert_eq!(saved["status"], 200);
            let entry = json!({"id":"","date":"2026-10-12","title":"First progress","notes":"Getting there","photos":["data:image/png;base64,aGVsbG8="],"author":""});
            let saved=store.call("action",token,Some(json!({"revision":2,"action":{"type":"add_entry","car_id":cid,"entry":entry}}))).await;
            assert_eq!(saved["status"], 200);
        }
        {
            let reopened = DeviceGarage::open(&path).unwrap();
            let opened = reopened.call("device/open", "", None).await;
            let car = &opened["body"]["snapshot"]["garage"]["cars"][0];
            assert_eq!(car["name"], "Weekend build");
            assert_eq!(car["items"][0]["price_cents"], 24995);
            assert_eq!(car["items"][0]["purchased_on"], "2026-10-12");
            assert_eq!(car["items"][0]["receipt_name"], "receipt.png");
            assert_eq!(
                car["journal"][0]["photos"][0],
                "data:image/png;base64,aGVsbG8="
            );
        }
        // Delete only this test's known temporary database files.
        for suffix in ["garage.db", "garage.db-wal", "garage.db-shm"] {
            let file = directory.join(suffix);
            if file.exists() {
                std::fs::remove_file(file).unwrap();
            }
        }
        std::fs::remove_dir(directory).unwrap();
    }
    #[tokio::test]
    async fn shared_garage_and_conflict_checks() {
        let app = router(App {
            db: Arc::new(Mutex::new(database(":memory:"))),
            attempts: Default::default(),
        });
        let (s, owner) = request(
            app.clone(),
            "/api/register",
            "",
            Some(
                json!({"name":"Kevin","email":"kevin@example.com","password":"long-test-password"}),
            ),
        )
        .await;
        assert_eq!(s, StatusCode::OK);
        let token = owner["token"].as_str().unwrap();
        let (_, inv) = request(app.clone(), "/api/invite", token, Some(json!({}))).await;
        let(s,staff)=request(app.clone(),"/api/register","",Some(json!({"name":"Sam","email":"sam@example.com","password":"long-test-password","invite":inv["code"]}))).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(staff["snapshot"]["user"]["role"], "staff");
        assert_eq!(
            staff["snapshot"]["garage"]["name"],
            "Kevin's Project Garage"
        );
        let action = json!({"revision":0,"action":{"type":"configure","name":"The Workshop","currency":"USD"}});
        let (s, _) = request(
            app.clone(),
            "/api/action",
            staff["token"].as_str().unwrap(),
            Some(action.clone()),
        )
        .await;
        assert_eq!(s, StatusCode::FORBIDDEN);
        let (s, _) = request(app.clone(), "/api/action", token, Some(action.clone())).await;
        assert_eq!(s, StatusCode::OK);
        let (s, _) = request(app.clone(), "/api/action", token, Some(action)).await;
        assert_eq!(s, StatusCode::CONFLICT);
        let (s, _) = request(app.clone(), "/api/garage", "wrong", None).await;
        assert_eq!(s, StatusCode::UNAUTHORIZED);
        let (s, _) = request(
            app.clone(),
            "/api/login",
            "",
            Some(json!({"email":"kevin@example.com","password":"wrong"})),
        )
        .await;
        assert_eq!(s, StatusCode::UNAUTHORIZED);
        let (s, _) = request(
            app,
            "/api/login",
            "",
            Some(json!({"email":"kevin@example.com","password":"long-test-password"})),
        )
        .await;
        assert_eq!(s, StatusCode::OK);
    }
}
