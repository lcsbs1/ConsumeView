//! OS keyring for API keys, with local-file fallback for headless Linux.

use anyhow::{Context, Result};
use keyring::Entry;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const SERVICE: &str = "ConsumeView";
const GEMINI_USER: &str = "GEMINI_API_KEY";
const XAI_USER: &str = "XAI_API_KEY";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct KeyStatus {
    pub gemini_set: bool,
    pub xai_set: bool,
}

fn fallback_path() -> PathBuf {
    let base = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("ConsumeView").join("secrets.local.json")
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct FallbackStore {
    gemini: Option<String>,
    xai: Option<String>,
}

fn read_fallback() -> FallbackStore {
    let path = fallback_path();
    fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn write_fallback(store: &FallbackStore) -> Result<()> {
    let path = fallback_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    // Restrictive permissions where supported
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        use std::io::Write;
        let mut f = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .mode(0o600)
            .open(&path)?;
        f.write_all(serde_json::to_string_pretty(store)?.as_bytes())?;
    }
    #[cfg(not(unix))]
    {
        fs::write(&path, serde_json::to_string_pretty(store)?)?;
    }
    Ok(())
}

fn set_keyring(user: &str, value: &str) -> Result<()> {
    let entry = Entry::new(SERVICE, user).context("keyring entry")?;
    entry.set_password(value).context("keyring set")?;
    Ok(())
}

fn get_keyring(user: &str) -> Option<String> {
    Entry::new(SERVICE, user)
        .ok()
        .and_then(|e| e.get_password().ok())
        .filter(|s| !s.is_empty())
}

fn delete_keyring(user: &str) {
    if let Ok(entry) = Entry::new(SERVICE, user) {
        let _ = entry.delete_credential();
    }
}

pub fn set_gemini_key(value: &str) -> Result<()> {
    match set_keyring(GEMINI_USER, value) {
        Ok(()) => {
            let mut fb = read_fallback();
            fb.gemini = None;
            let _ = write_fallback(&fb);
            Ok(())
        }
        Err(_) => {
            let mut fb = read_fallback();
            fb.gemini = Some(value.to_string());
            write_fallback(&fb)
        }
    }
}

pub fn set_xai_key(value: &str) -> Result<()> {
    match set_keyring(XAI_USER, value) {
        Ok(()) => {
            let mut fb = read_fallback();
            fb.xai = None;
            let _ = write_fallback(&fb);
            Ok(())
        }
        Err(_) => {
            let mut fb = read_fallback();
            fb.xai = Some(value.to_string());
            write_fallback(&fb)
        }
    }
}

pub fn get_gemini_key() -> Option<String> {
    get_keyring(GEMINI_USER).or_else(|| read_fallback().gemini)
}

pub fn get_xai_key() -> Option<String> {
    get_keyring(XAI_USER).or_else(|| read_fallback().xai)
}

pub fn clear_gemini_key() -> Result<()> {
    delete_keyring(GEMINI_USER);
    let mut fb = read_fallback();
    fb.gemini = None;
    write_fallback(&fb)
}

pub fn clear_xai_key() -> Result<()> {
    delete_keyring(XAI_USER);
    let mut fb = read_fallback();
    fb.xai = None;
    write_fallback(&fb)
}

pub fn key_status() -> KeyStatus {
    KeyStatus {
        gemini_set: get_gemini_key().is_some(),
        xai_set: get_xai_key().is_some(),
    }
}
