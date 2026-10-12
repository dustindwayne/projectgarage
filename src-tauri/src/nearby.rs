use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    net::UdpSocket,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
const PORT: u16 = 39073;
#[derive(Serialize, Deserialize)]
struct Beacon {
    protocol: String,
    device_id: String,
    platform: String,
}
#[derive(Clone, Serialize)]
pub struct Peer {
    pub id: String,
    pub address: String,
    pub name: String,
}
#[derive(Clone, Serialize)]
pub struct Snapshot {
    pub platform: String,
    pub enabled: bool,
    pub peers: Vec<Peer>,
    pub error: Option<String>,
}
pub struct Nearby {
    state: Arc<Mutex<Snapshot>>,
    enabled: Arc<std::sync::atomic::AtomicBool>,
}
impl Nearby {
    pub fn start(initially_enabled: bool) -> Self {
        let android = cfg!(target_os = "android");
        let state = Arc::new(Mutex::new(Snapshot {
            platform: if android { "android" } else { "desktop" }.into(),
            enabled: initially_enabled,
            peers: vec![],
            error: None,
        }));
        let enabled = Arc::new(std::sync::atomic::AtomicBool::new(initially_enabled));
        let service = Self {
            state: state.clone(),
            enabled: enabled.clone(),
        };
        std::thread::spawn(move || {
            let socket = match UdpSocket::bind(if android {
                "0.0.0.0:0".into()
            } else {
                format!("0.0.0.0:{PORT}")
            }) {
                Ok(socket) => socket,
                Err(e) => {
                    state.lock().unwrap().error = Some(format!("Wi-Fi discovery unavailable: {e}"));
                    return;
                }
            };
            if android {
                if let Err(e) = socket.set_broadcast(true) {
                    state.lock().unwrap().error = Some(e.to_string());
                    return;
                }
                let message = serde_json::to_vec(&Beacon {
                    protocol: "projectgarage/1".into(),
                    device_id: uuid::Uuid::new_v4().to_string(),
                    platform: "android".into(),
                })
                .unwrap();
                loop {
                    if enabled.load(std::sync::atomic::Ordering::Relaxed) {
                        let error = socket
                            .send_to(&message, format!("255.255.255.255:{PORT}"))
                            .err()
                            .map(|e| format!("Could not announce on Wi-Fi: {e}"));
                        state.lock().unwrap().error = error;
                    }
                    std::thread::sleep(Duration::from_secs(3));
                }
            } else {
                let _ = socket.set_read_timeout(Some(Duration::from_secs(1)));
                let mut discovered: HashMap<String, (Peer, Instant)> = HashMap::new();
                let mut buffer = [0u8; 512];
                loop {
                    if let Ok((size, address)) = socket.recv_from(&mut buffer) {
                        if enabled.load(std::sync::atomic::Ordering::Relaxed) {
                            if let Some(id) = parse_beacon(&buffer[..size]) {
                                if discovered.len() >= 64 && !discovered.contains_key(&id) {
                                    continue;
                                }
                                discovered.insert(
                                    id.clone(),
                                    (
                                        Peer {
                                            id,
                                            address: address.ip().to_string(),
                                            name: "Project Garage on Android".into(),
                                        },
                                        Instant::now(),
                                    ),
                                );
                            }
                        }
                    }
                    discovered.retain(|_, (_, seen)| seen.elapsed() < Duration::from_secs(12));
                    let mut current = state.lock().unwrap();
                    current.peers = if current.enabled {
                        discovered.values().map(|(peer, _)| peer.clone()).collect()
                    } else {
                        vec![]
                    };
                    current.peers.sort_by(|a, b| a.address.cmp(&b.address));
                }
            }
        });
        service
    }
    pub fn snapshot(&self) -> Snapshot {
        self.state.lock().unwrap().clone()
    }
    pub fn set_enabled(&self, value: bool) -> Snapshot {
        self.enabled
            .store(value, std::sync::atomic::Ordering::Relaxed);
        let mut state = self.state.lock().unwrap();
        state.enabled = value;
        if !value {
            state.peers.clear();
        }
        state.clone()
    }
}
fn parse_beacon(bytes: &[u8]) -> Option<String> {
    let beacon: Beacon = serde_json::from_slice(bytes).ok()?;
    if beacon.protocol != "projectgarage/1"
        || beacon.platform != "android"
        || uuid::Uuid::parse_str(&beacon.device_id).is_err()
    {
        return None;
    }
    Some(beacon.device_id)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn accepts_only_android_app_beacons() {
        let valid = serde_json::to_vec(&Beacon {
            protocol: "projectgarage/1".into(),
            device_id: uuid::Uuid::new_v4().to_string(),
            platform: "android".into(),
        })
        .unwrap();
        assert!(parse_beacon(&valid).is_some());
        assert!(parse_beacon(b"not json").is_none());
        assert!(parse_beacon(
            br#"{"protocol":"projectgarage/1","device_id":"bad","platform":"android"}"#
        )
        .is_none());
    }
    #[test]
    #[cfg(not(target_os = "android"))]
    fn desktop_detects_and_can_disable_an_android_announcement() {
        let nearby = Nearby::start(true);
        let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        let id = uuid::Uuid::new_v4().to_string();
        let message = serde_json::to_vec(&Beacon {
            protocol: "projectgarage/1".into(),
            device_id: id.clone(),
            platform: "android".into(),
        })
        .unwrap();
        let started = Instant::now();
        while started.elapsed() < Duration::from_secs(3) && nearby.snapshot().peers.is_empty() {
            socket.send_to(&message, ("127.0.0.1", PORT)).unwrap();
            std::thread::sleep(Duration::from_millis(100));
        }
        assert!(nearby.snapshot().peers.iter().any(|peer| peer.id == id));
        assert!(nearby.set_enabled(false).peers.is_empty());
    }
}
