use aes_gcm::{
    aead::{Aead, AeadCore, OsRng},
    Aes256Gcm, KeyInit, Nonce,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream, UdpSocket},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use x25519_dalek::{PublicKey, StaticSecret};
const MAX: usize = 100_000_000;
#[derive(Clone, Default, Serialize)]
pub struct View {
    pub status: String,
    pub code: String,
    pub address: String,
    pub request_id: String,
    pub error: String,
}
struct Pending {
    view: View,
    key: [u8; 32],
    endpoint: SocketAddr,
    created: Instant,
}
#[derive(Clone, Default)]
pub struct Transfer {
    view: Arc<Mutex<View>>,
    pending: Arc<Mutex<Option<Pending>>>,
    backup: Arc<Mutex<Option<String>>>,
}
#[derive(Serialize, Deserialize)]
struct Request {
    protocol: String,
    id: String,
    public: [u8; 32],
    port: u16,
}
#[derive(Serialize, Deserialize)]
struct Reply {
    id: String,
    public: [u8; 32],
}
fn key(secret: &StaticSecret, public: [u8; 32], id: &str) -> Result<[u8; 32], String> {
    let shared = secret.diffie_hellman(&PublicKey::from(public));
    if !shared.was_contributory() {
        return Err("Invalid connection key".into());
    }
    let mut h = Sha256::new();
    h.update(b"projectgarage-transfer/1");
    h.update(shared.as_bytes());
    h.update(id.as_bytes());
    Ok(h.finalize().into())
}
fn code(key: &[u8; 32]) -> String {
    let mut h = Sha256::new();
    h.update(b"verify pairing");
    h.update(key);
    format!("{:x}", h.finalize())[..8].to_uppercase()
}
impl Transfer {
    pub fn view(&self) -> View {
        let mut p = self.pending.lock().unwrap();
        if p.as_ref()
            .is_some_and(|p| p.created.elapsed() > Duration::from_secs(90))
        {
            *p = None;
            *self.view.lock().unwrap() = View {
                status: "expired".into(),
                ..View::default()
            };
        }
        self.view.lock().unwrap().clone()
    }
    pub fn receive_request(&self, data: &[u8], source: SocketAddr, socket: &UdpSocket) {
        let Ok(r) = serde_json::from_slice::<Request>(data) else {
            return;
        };
        if r.protocol != "projectgarage-transfer/1"
            || uuid::Uuid::parse_str(&r.id).is_err()
            || r.port == 0
        {
            return;
        }
        let mut pending = self.pending.lock().unwrap();
        if pending
            .as_ref()
            .is_some_and(|p| p.created.elapsed() < Duration::from_secs(90))
        {
            return;
        }
        let secret = StaticSecret::random();
        let Ok(k) = key(&secret, r.public, &r.id) else {
            return;
        };
        let view = View {
            status: "approval".into(),
            code: code(&k),
            address: source.ip().to_string(),
            request_id: r.id.clone(),
            error: String::new(),
        };
        *pending = Some(Pending {
            view: view.clone(),
            key: k,
            endpoint: SocketAddr::new(source.ip(), r.port),
            created: Instant::now(),
        });
        *self.view.lock().unwrap() = view;
        let reply = Reply {
            id: r.id,
            public: PublicKey::from(&secret).to_bytes(),
        };
        let _ = socket.send_to(&serde_json::to_vec(&reply).unwrap(), source);
    }
    pub fn request(&self, address: String) -> Result<View, String> {
        self.request_on_port(address, 39073)
    }
    fn request_on_port(&self, address: String, phone_port: u16) -> Result<View, String> {
        if cfg!(target_os = "android") {
            return Err("Choose a phone from the desktop app".into());
        }
        if ["waiting", "verify"].contains(&self.view.lock().unwrap().status.as_str()) {
            return Err("A transfer is already pending".into());
        }
        let ip: std::net::IpAddr = address.parse().map_err(|_| "Invalid phone address")?;
        let listener = TcpListener::bind("0.0.0.0:0").map_err(|e| e.to_string())?;
        listener.set_nonblocking(true).map_err(|e| e.to_string())?;
        let udp = UdpSocket::bind("0.0.0.0:0").map_err(|e| e.to_string())?;
        udp.set_read_timeout(Some(Duration::from_secs(10)))
            .map_err(|e| e.to_string())?;
        let secret = StaticSecret::random();
        let id = uuid::Uuid::new_v4().to_string();
        let r = Request {
            protocol: "projectgarage-transfer/1".into(),
            id: id.clone(),
            public: PublicKey::from(&secret).to_bytes(),
            port: listener.local_addr().unwrap().port(),
        };
        *self.backup.lock().unwrap() = None;
        *self.view.lock().unwrap() = View {
            status: "waiting".into(),
            address: address.clone(),
            request_id: id.clone(),
            ..View::default()
        };
        let service = self.clone();
        std::thread::spawn(move || {
            let result = (|| -> Result<String, String> {
                udp.send_to(
                    &serde_json::to_vec(&r).unwrap(),
                    SocketAddr::new(ip, phone_port),
                )
                .map_err(|e| e.to_string())?;
                let mut buf = [0; 2048];
                let (n, from) = udp.recv_from(&mut buf).map_err(|_| {
                    "Phone did not respond. Keep it open and check Wi-Fi/firewall settings."
                })?;
                if from.ip() != ip {
                    return Err("Unexpected phone response".into());
                }
                let reply: Reply =
                    serde_json::from_slice(&buf[..n]).map_err(|_| "Invalid phone response")?;
                if reply.id != id {
                    return Err("Transfer session mismatch".into());
                }
                let k = key(&secret, reply.public, &id)?;
                {
                    let mut v = service.view.lock().unwrap();
                    v.status = "verify".into();
                    v.code = code(&k);
                }
                let start = Instant::now();
                let mut stream = loop {
                    match listener.accept() {
                        Ok((s, peer)) => {
                            if peer.ip() == ip {
                                break s;
                            }
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                        Err(e) => return Err(e.to_string()),
                    };
                    if service.view.lock().unwrap().request_id != id {
                        return Err("Transfer cancelled".into());
                    }
                    if start.elapsed() > Duration::from_secs(90) {
                        return Err("Transfer expired. Try again.".into());
                    }
                    std::thread::sleep(Duration::from_millis(100));
                };
                stream
                    .set_read_timeout(Some(Duration::from_secs(30)))
                    .map_err(|e| e.to_string())?;
                let mut size = [0; 8];
                stream.read_exact(&mut size).map_err(|e| e.to_string())?;
                let size = u64::from_be_bytes(size);
                if size > MAX as u64 + 28 || size < 28 {
                    return Err("Invalid backup size".into());
                }
                let mut bytes = vec![0; size as usize];
                stream.read_exact(&mut bytes).map_err(|e| e.to_string())?;
                let plain = Aes256Gcm::new_from_slice(&k)
                    .unwrap()
                    .decrypt(Nonce::from_slice(&bytes[..12]), &bytes[12..])
                    .map_err(|_| "Transfer integrity check failed")?;
                let text = String::from_utf8(plain).map_err(|_| "Invalid backup encoding")?;
                let value: serde_json::Value =
                    serde_json::from_str(&text).map_err(|_| "Invalid backup")?;
                if value.get("garage").is_none() {
                    return Err("Missing garage records".into());
                }
                let _: garage_core::Garage = serde_json::from_value(value["garage"].clone())
                    .map_err(|_| "Invalid garage records")?;
                Ok(text)
            })();
            match result {
                Ok(text) => {
                    *service.backup.lock().unwrap() = Some(text);
                    service.view.lock().unwrap().status = "ready".into();
                }
                Err(error) => {
                    let mut v = service.view.lock().unwrap();
                    v.status = "error".into();
                    v.error = error;
                }
            }
        });
        Ok(self.view())
    }
    pub fn approve(&self, id: String, contents: String) -> Result<View, String> {
        if contents.len() > MAX {
            return Err("Garage exceeds transfer limit".into());
        }
        let pending = self
            .pending
            .lock()
            .unwrap()
            .take()
            .ok_or("No transfer is pending")?;
        if pending.view.request_id != id || pending.created.elapsed() > Duration::from_secs(90) {
            return Err("Transfer expired".into());
        }
        let cipher = Aes256Gcm::new_from_slice(&pending.key).unwrap();
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let encrypted = cipher
            .encrypt(&nonce, contents.as_bytes())
            .map_err(|_| "Could not encrypt backup")?;
        let result = (|| -> Result<(), String> {
            let mut s = TcpStream::connect_timeout(&pending.endpoint, Duration::from_secs(10))
                .map_err(|e| e.to_string())?;
            s.set_write_timeout(Some(Duration::from_secs(30)))
                .map_err(|e| e.to_string())?;
            s.write_all(&((encrypted.len() + 12) as u64).to_be_bytes())
                .map_err(|e| e.to_string())?;
            s.write_all(&nonce).map_err(|e| e.to_string())?;
            s.write_all(&encrypted).map_err(|e| e.to_string())?;
            Ok(())
        })();
        let mut v = self.view.lock().unwrap();
        v.status = if result.is_ok() { "sent" } else { "error" }.into();
        v.error = result.err().unwrap_or_default();
        Ok(v.clone())
    }
    pub fn reject(&self) {
        *self.pending.lock().unwrap() = None;
        *self.view.lock().unwrap() = View {
            status: "declined".into(),
            ..View::default()
        };
    }
    pub fn take(&self) -> Result<String, String> {
        let text = self
            .backup
            .lock()
            .unwrap()
            .take()
            .ok_or("No transferred garage is ready")?;
        self.view.lock().unwrap().status = "review".into();
        Ok(text)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn authenticated_transfer_keys_match_and_tampering_fails() {
        let a = StaticSecret::random();
        let b = StaticSecret::random();
        let ka = key(&a, PublicKey::from(&b).to_bytes(), "session").unwrap();
        let kb = key(&b, PublicKey::from(&a).to_bytes(), "session").unwrap();
        assert_eq!(ka, kb);
        assert_eq!(code(&ka), code(&kb));
        assert_ne!(
            ka,
            key(&a, PublicKey::from(&b).to_bytes(), "other").unwrap()
        );
        assert!(key(&a, [0; 32], "session").is_err());
        let cipher = Aes256Gcm::new_from_slice(&ka).unwrap();
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let mut data = cipher.encrypt(&nonce, b"records".as_ref()).unwrap();
        assert_eq!(cipher.decrypt(&nonce, data.as_ref()).unwrap(), b"records");
        data[0] ^= 1;
        assert!(cipher.decrypt(&nonce, data.as_ref()).is_err());
    }
    #[test]
    fn desktop_receives_only_phone_approved_backup() {
        let desktop = Transfer::default();
        let phone = Transfer::default();
        let udp = UdpSocket::bind("127.0.0.1:0").unwrap();
        udp.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
        desktop
            .request_on_port("127.0.0.1".into(), udp.local_addr().unwrap().port())
            .unwrap();
        let mut buf = [0; 2048];
        let (n, source) = udp.recv_from(&mut buf).unwrap();
        phone.receive_request(&buf[..n], source, &udp);
        let started = Instant::now();
        while desktop.view().status != "verify" && started.elapsed() < Duration::from_secs(3) {
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(desktop.view().code, phone.view().code);
        assert!(desktop.take().is_err());
        let content=serde_json::json!({"garage":{"name":"Phone garage","currency":"USD","cars":[],"padding":"x".repeat(2_000_000)}}).to_string();
        let result = phone
            .approve(phone.view().request_id, content.clone())
            .unwrap();
        assert_eq!(result.status, "sent");
        let started = Instant::now();
        while desktop.view().status != "ready" && started.elapsed() < Duration::from_secs(5) {
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(desktop.take().unwrap(), content);
        assert!(desktop.take().is_err());
        assert!(phone.approve(result.request_id, "{}".into()).is_err());
    }
}
