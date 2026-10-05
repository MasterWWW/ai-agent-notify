// Minimal structured logger. Never logs tokens, secrets, or full payloads.
use std::fmt::Write as _;

fn ts() -> String {
    use chrono::Local;
    Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

pub fn log(fields: &[(&str, Option<&str>)]) {
    let mut parts = String::new();
    let _ = write!(parts, "{}", ts());
    for (k, v) in fields {
        if let Some(v) = v {
            let _ = write!(parts, " {k}={v}");
        }
    }
    println!("{parts}");
}

pub fn log_simple(key: &str, value: &str) {
    log(&[(key, Some(value))]);
}

pub fn error(msg: &str, detail: Option<&str>) {
    let d = detail.unwrap_or("");
    if d.is_empty() {
        eprintln!("{} error=\"{msg}\"", ts());
    } else {
        eprintln!("{} error=\"{msg}\" detail=\"{d}\"", ts());
    }
}
