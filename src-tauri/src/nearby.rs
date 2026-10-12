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
    pub fn start(initially_enabled: bool, transfer: crate::transfer::Transfer) -> Self {
        Self::start_on_port(initially_enabled, transfer, PORT)
    }
    fn start_on_port(
        initially_enabled: bool,
        transfer: crate::transfer::Transfer,
        port: u16,
    ) -> Self {
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
                format!("0.0.0.0:{port}")
            } else {
                format!("0.0.0.0:{port}")
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
                let _ = socket.set_read_timeout(Some(Duration::from_millis(200)));
                let mut last = Instant::now() - Duration::from_secs(3);
                let mut request = [0u8; 2048];
                loop {
                    if let Ok((size, source)) = socket.recv_from(&mut request) {
                        if enabled.load(std::sync::atomic::Ordering::Relaxed) {
                            transfer.receive_request(&request[..size], source, &socket);
                        }
                    }
                    if enabled.load(std::sync::atomic::Ordering::Relaxed)
                        && last.elapsed() > Duration::from_secs(3)
                    {
                        last = Instant::now();
                        let error = socket
                            .send_to(&message, format!("255.255.255.255:{PORT}"))
                            .err()
                            .map(|e| format!("Could not announce on Wi-Fi: {e}"));
                        state.lock().unwrap().error = error;
                    }
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
        let reserve = UdpSocket::bind("127.0.0.1:0").unwrap();
        let port = reserve.local_addr().unwrap().port();
        drop(reserve);
        let nearby = Nearby::start_on_port(true, crate::transfer::Transfer::default(), port);
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
            socket.send_to(&message, ("127.0.0.1", port)).unwrap();
            std::thread::sleep(Duration::from_millis(100));
        }
        assert!(nearby.snapshot().peers.iter().any(|peer| peer.id == id));
        assert!(nearby.set_enabled(false).peers.is_empty());
    }
}
