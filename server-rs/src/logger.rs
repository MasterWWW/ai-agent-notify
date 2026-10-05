// Minimal structured logger. Never logs tokens, secrets, or full payloads.
// Writes to stdout/stderr AND appends to ~/.ai-task-notify/server.log so
// diagnostics survive when the process runs as a GUI app (open) with no
// visible terminal.
use std::fmt::Write as _;
use std::fs::OpenOptions;
use std::io::Write;

fn ts() -> String {
    use chrono::Local;
    Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

fn append_file(line: &str) {
    let dir = crate::state::state_dir();
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(dir.join("server.log")) {
        let _ = writeln!(f, "{line}");
    }
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
    append_file(&parts);
}

pub fn log_simple(key: &str, value: &str) {
    log(&[(key, Some(value))]);
}

pub fn error(msg: &str, detail: Option<&str>) {
    let d = detail.unwrap_or("");
    let line = if d.is_empty() {
        format!("{} error=\"{msg}\"", ts())
    } else {
        format!("{} error=\"{msg}\" detail=\"{d}\"", ts())
    };
    eprintln!("{line}");
    append_file(&line);
}
