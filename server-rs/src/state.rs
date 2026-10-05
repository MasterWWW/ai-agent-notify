use std::fs;
use std::os::unix::fs::OpenOptionsExt;
use std::path::PathBuf;

pub const STATE_DIR_NAME: &str = ".ai-task-notify";

pub fn state_dir() -> PathBuf {
    home_dir().join(STATE_DIR_NAME)
}
pub fn token_file() -> PathBuf {
    state_dir().join("token")
}
pub fn config_file() -> PathBuf {
    state_dir().join("config.json")
}
pub fn events_file() -> PathBuf {
    state_dir().join("events.jsonl")
}
pub fn log_file() -> PathBuf {
    state_dir().join("server.log")
}

pub fn home_dir() -> PathBuf {
    home::home_dir().unwrap_or_else(|| PathBuf::from("."))
}

fn ensure_state_dir() -> std::io::Result<()> {
    fs::create_dir_all(state_dir())
}

fn read_token_file() -> Option<String> {
    let f = token_file();
    if !f.exists() {
        return None;
    }
    match fs::read_to_string(&f) {
        Ok(t) => {
            let t = t.trim().to_string();
            if t.is_empty() {
                None
            } else {
                Some(t)
            }
        }
        Err(_) => None,
    }
}

fn write_token_file(token: &str) {
    if let Err(e) = ensure_state_dir() {
        eprintln!("state dir create failed (best effort): {e}");
        return;
    }
    if let Ok(mut f) = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(token_file())
    {
        use std::io::Write;
        let _ = f.write_all(token.as_bytes());
        let _ = f.write_all(b"\n");
    }
}

fn generate_token() -> String {
    crate::model::random_hex(24)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenSource {
    Cli,
    Env,
    File,
    Generated,
}

impl TokenSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            TokenSource::Cli => "cli",
            TokenSource::Env => "env",
            TokenSource::File => "file",
            TokenSource::Generated => "generated",
        }
    }
}

/// Resolve the server token with precedence: --token > env > file > generated.
pub fn resolve_token(cli_token: Option<String>) -> (String, TokenSource) {
    if let Some(t) = cli_token.filter(|t| !t.is_empty()) {
        return (t, TokenSource::Cli);
    }
    if let Ok(env) = std::env::var("AI_TASK_NOTIFY_TOKEN") {
        if !env.is_empty() {
            return (env, TokenSource::Env);
        }
    }
    if let Some(t) = read_token_file() {
        return (t, TokenSource::File);
    }
    let generated = generate_token();
    write_token_file(&generated);
    (generated, TokenSource::Generated)
}

/// Read the token for hook/CLI clients (does not generate).
pub fn read_token_for_hook(cli_token: Option<String>) -> Option<String> {
    if let Some(t) = cli_token.filter(|t| !t.is_empty()) {
        return Some(t);
    }
    if let Ok(env) = std::env::var("AI_TASK_NOTIFY_TOKEN") {
        if !env.is_empty() {
            return Some(env);
        }
    }
    read_token_file()
}

/// Constant-time token comparison (mirrors Node timingSafeEqual).
pub fn token_eq(a: &str, b: &str) -> bool {
    let a = a.as_bytes();
    let b = b.as_bytes();
    if a.len() != b.len() {
        return false;
    }
    let mut diff: u8 = 0;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}
