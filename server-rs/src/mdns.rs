// mDNS / hostname / LAN IP helpers (mirrors the Node transport/mdns.ts).
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::sync::OnceLock;

fn dns_sd_child() -> &'static Mutex<Option<std::process::Child>> {
    static CHILD: OnceLock<Mutex<Option<std::process::Child>>> = OnceLock::new();
    CHILD.get_or_init(|| Mutex::new(None))
}

/// Resolve the friendly mDNS hostname advertised by macOS
/// (e.g. weichaoyingdeMac-mini.local). Falls back to os hostname + .local.
pub fn get_local_hostname() -> String {
    #[cfg(target_os = "macos")]
    {
        if let Ok(out) = Command::new("/usr/sbin/scutil")
            .args(["--get", "LocalHostName"])
            .output()
        {
            let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !s.is_empty() {
                return format!("{s}.local");
            }
        }
    }
    let base = std::env::var("HOSTNAME")
        .unwrap_or_else(|_| "unknown".to_string())
        .trim_end_matches(".local")
        .trim_end_matches(".lan")
        .to_string();
    format!("{base}.local")
}

fn is_private_lan(ip: &str) -> bool {
    let parts: Vec<u32> = ip.split('.').filter_map(|p| p.parse().ok()).collect();
    if parts.len() != 4 {
        return false;
    }
    let (a, b) = (parts[0], parts[1]);
    (a == 10) || (a == 172 && (16..=31).contains(&b)) || (a == 192 && b == 168)
}

/// All non-internal IPv4 LAN addresses (private ranges only), sorted.
pub fn get_lan_ips() -> Vec<String> {
    let mut out = std::collections::BTreeSet::new();
    if let Ok(ifaces) = if_addrs::get_if_addrs() {
        for iface in ifaces {
            if iface.is_loopback() {
                continue;
            }
            if let if_addrs::IfAddr::V4(v4) = iface.addr {
                let ip = v4.ip.to_string();
                if is_private_lan(&ip) {
                    out.insert(ip);
                }
            }
        }
    }
    out.into_iter().collect()
}

/// Register a Bonjour service via the macOS native `dns-sd` daemon
/// (mirrors Node `publishBonjourService`).
pub fn publish_bonjour_service(name: &str, service_type: &str, port: u16) {
    #[cfg(target_os = "macos")]
    {
        let mut guard = dns_sd_child().lock().unwrap();
        if guard.is_some() {
            return;
        }
        match Command::new("dns-sd")
            .args(["-R", name, &format!("_{service_type}._tcp"), "local", &port.to_string()])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(child) => {
                *guard = Some(child);
                crate::logger::log(&[
                    ("msg", Some("bonjour service registered")),
                    ("service", Some(&format!("{service_type}.local"))),
                    ("name", Some(name)),
                    ("port", Some(&port.to_string())),
                ]);
            }
            Err(e) => {
                crate::logger::log(&[
                    ("msg", Some("bonjour publish failed (continuing without it)")),
                    ("error", Some(&e.to_string())),
                ]);
            }
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (name, service_type, port);
        crate::logger::log(&[
            ("msg", Some("bonjour skipped (macOS only)")),
        ]);
    }
}

pub fn stop_bonjour() {
    let mut guard = dns_sd_child().lock().unwrap();
    if let Some(mut child) = guard.take() {
        let _ = child.kill();
        let _ = child.wait();
    }
}
