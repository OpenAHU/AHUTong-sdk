use anyhow::{Context, Result};

#[cfg(not(target_arch = "wasm32"))]
use guixu::{GuiXu, GuiXuError};
#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::{Mutex, OnceLock};

#[cfg(not(target_arch = "wasm32"))]
const SESSION_BOX: &str = "session";
#[cfg(not(target_arch = "wasm32"))]
const COOKIES_KEY: &str = "cookies_json";

#[cfg(not(target_arch = "wasm32"))]
struct Persistence {
    db: GuiXu,
    path: PathBuf,
}

#[cfg(not(target_arch = "wasm32"))]
static PERSISTENCE: OnceLock<Mutex<Option<Persistence>>> = OnceLock::new();

#[cfg(not(target_arch = "wasm32"))]
fn persistence_cell() -> &'static Mutex<Option<Persistence>> {
    PERSISTENCE.get_or_init(|| Mutex::new(None))
}

pub fn init(storage_path: &str, seed_cookies_json: &str) -> Result<Option<String>> {
    #[cfg(target_arch = "wasm32")]
    {
        Ok(None)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let storage_path = storage_path.trim();
        if storage_path.is_empty() {
            return Ok(None);
        }

        let path = PathBuf::from(storage_path);
        let db = GuiXu::new(path.clone())
            .with_context(|| format!("failed to open Rust persistence at {}", path.display()))?;
        let stored_cookies = read_string_from(&db, SESSION_BOX, COOKIES_KEY)
            .context("failed to read persisted Rust cookies")?;

        let cookies_to_restore = if stored_cookies.is_none() && !seed_cookies_json.trim().is_empty() {
            write_string_to(&db, SESSION_BOX, COOKIES_KEY, seed_cookies_json)
                .context("failed to migrate seeded Rust cookies")?;
            Some(seed_cookies_json.to_string())
        } else {
            stored_cookies
        };

        let mut guard = persistence_cell()
            .lock()
            .expect("persistence mutex poisoned");
        *guard = Some(Persistence { db, path });

        Ok(cookies_to_restore)
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn validate_name(kind: &str, value: &str) -> Result<()> {
    if value.is_empty() {
        anyhow::bail!("{kind} cannot be empty");
    }
    if value.contains('/')
        || value.contains('\\')
        || value.contains(':')
        || value.contains("..")
        || value.chars().any(|c| c.is_control())
    {
        anyhow::bail!("invalid {kind}: {value}");
    }
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
fn read_string_from(db: &GuiXu, box_name: &str, key: &str) -> Result<Option<String>> {
    validate_name("box name", box_name)?;
    validate_name("key", key)?;
    let mut kv = db
        .kv_box_for(box_name)
        .with_context(|| format!("failed to open GuiXu box {box_name}"))?;
    let value = match kv.get_string(key) {
        Ok(cookies) if !cookies.is_empty() => Some(cookies.to_string()),
        Ok(_) | Err(GuiXuError::KeyNotFound(_)) => None,
        Err(err) => return Err(err).context("failed to read persisted Rust cookies"),
    };
    kv.close()
        .with_context(|| format!("failed to close GuiXu box {box_name}"))?;
    Ok(value)
}

#[cfg(not(target_arch = "wasm32"))]
fn write_string_to(db: &GuiXu, box_name: &str, key: &str, value: &str) -> Result<()> {
    validate_name("box name", box_name)?;
    validate_name("key", key)?;
    let mut kv = db
        .kv_box_for(box_name)
        .with_context(|| format!("failed to open GuiXu box {box_name}"))?;
    kv.put_string(key, value.to_string())
        .with_context(|| format!("failed to write key {key} in GuiXu box {box_name}"))?;
    kv.close()
        .with_context(|| format!("failed to flush GuiXu box {box_name}"))?;
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
fn remove_from(db: &GuiXu, box_name: &str, key: &str) -> Result<()> {
    validate_name("box name", box_name)?;
    validate_name("key", key)?;
    let mut kv = db
        .kv_box_for(box_name)
        .with_context(|| format!("failed to open GuiXu box {box_name}"))?;
    kv.remove(key)
        .with_context(|| format!("failed to remove key {key} in GuiXu box {box_name}"))?;
    kv.close()
        .with_context(|| format!("failed to flush GuiXu box {box_name}"))?;
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
fn clear_box_in(db: &GuiXu, box_name: &str) -> Result<()> {
    validate_name("box name", box_name)?;
    let mut kv = db
        .kv_box_for(box_name)
        .with_context(|| format!("failed to open GuiXu box {box_name}"))?;
    kv.clear(true)
        .with_context(|| format!("failed to clear GuiXu box {box_name}"))?;
    kv.close()
        .with_context(|| format!("failed to flush GuiXu box {box_name}"))?;
    Ok(())
}

pub fn save_cookies(cookies_json: &str) -> Result<bool> {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = cookies_json;
        Ok(false)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let guard = persistence_cell()
            .lock()
            .expect("persistence mutex poisoned");
        let Some(persistence) = guard.as_ref() else {
            return Ok(false);
        };

        if cookies_json.is_empty() {
            remove_from(&persistence.db, SESSION_BOX, COOKIES_KEY).with_context(|| {
                format!(
                    "failed to clear Rust cookies at {}",
                    persistence.path.display()
                )
            })?;
        } else {
            write_string_to(&persistence.db, SESSION_BOX, COOKIES_KEY, cookies_json).with_context(
                || {
                    format!(
                        "failed to persist Rust cookies at {}",
                        persistence.path.display()
                    )
                },
            )?;
        }
        Ok(true)
    }
}

pub fn clear_cookies() -> Result<bool> {
    save_cookies("")
}

pub fn put_string(box_name: &str, key: &str, value: &str) -> Result<bool> {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (box_name, key, value);
        Ok(false)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let guard = persistence_cell()
            .lock()
            .expect("persistence mutex poisoned");
        let Some(persistence) = guard.as_ref() else {
            return Ok(false);
        };
        write_string_to(&persistence.db, box_name, key, value)?;
        Ok(true)
    }
}

pub fn get_string(box_name: &str, key: &str) -> Result<Option<String>> {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (box_name, key);
        Ok(None)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let guard = persistence_cell()
            .lock()
            .expect("persistence mutex poisoned");
        let Some(persistence) = guard.as_ref() else {
            return Ok(None);
        };
        read_string_from(&persistence.db, box_name, key)
    }
}

pub fn remove_key(box_name: &str, key: &str) -> Result<bool> {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (box_name, key);
        Ok(false)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let guard = persistence_cell()
            .lock()
            .expect("persistence mutex poisoned");
        let Some(persistence) = guard.as_ref() else {
            return Ok(false);
        };
        remove_from(&persistence.db, box_name, key)?;
        Ok(true)
    }
}

pub fn clear_box(box_name: &str) -> Result<bool> {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = box_name;
        Ok(false)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let guard = persistence_cell()
            .lock()
            .expect("persistence mutex poisoned");
        let Some(persistence) = guard.as_ref() else {
            return Ok(false);
        };
        clear_box_in(&persistence.db, box_name)?;
        Ok(true)
    }
}
