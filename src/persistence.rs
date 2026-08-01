use anyhow::Result;
#[cfg(not(target_arch = "wasm32"))]
use anyhow::anyhow;

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
    persist_session: bool,
}

#[cfg(not(target_arch = "wasm32"))]
static PERSISTENCE: OnceLock<Mutex<Option<Persistence>>> = OnceLock::new();

#[cfg(not(target_arch = "wasm32"))]
fn persistence_cell() -> &'static Mutex<Option<Persistence>> {
    PERSISTENCE.get_or_init(|| Mutex::new(None))
}

pub fn init(
    storage_path: &str,
    seed_cookies_json: &str,
    persist_session: bool,
) -> Result<Option<String>> {
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
        let db = GuiXu::new(path).map_err(|_| anyhow!("persistence_open_failed"))?;
        let cookies_to_restore = if persist_session {
            let stored_cookies = read_string_from(&db, SESSION_BOX, COOKIES_KEY)?;
            if stored_cookies.is_none() && !seed_cookies_json.trim().is_empty() {
                write_string_to(&db, SESSION_BOX, COOKIES_KEY, seed_cookies_json)
                    .map_err(|_| anyhow!("persistence_cookie_migration_failed"))?;
                Some(seed_cookies_json.to_string())
            } else {
                stored_cookies
            }
        } else if seed_cookies_json.trim().is_empty() {
            None
        } else {
            // Apple clients keep Cookie/Token material in Keychain. The seed is
            // loaded into the in-memory crawler, but never written to GuiXu.
            Some(seed_cookies_json.to_string())
        };

        let mut guard = persistence_cell()
            .lock()
            .expect("persistence mutex poisoned");
        *guard = Some(Persistence {
            db,
            persist_session,
        });

        Ok(cookies_to_restore)
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn validate_name(kind: &str, value: &str) -> Result<()> {
    if value.is_empty() {
        return Err(anyhow!(match kind {
            "box name" => "persistence_box_name_empty",
            "key" => "persistence_key_empty",
            _ => "persistence_name_empty",
        }));
    }
    if value.contains('/')
        || value.contains('\\')
        || value.contains(':')
        || value.contains("..")
        || value.chars().any(|c| c.is_control())
    {
        return Err(anyhow!(match kind {
            "box name" => "persistence_box_name_invalid",
            "key" => "persistence_key_invalid",
            _ => "persistence_name_invalid",
        }));
    }
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
fn read_string_from(db: &GuiXu, box_name: &str, key: &str) -> Result<Option<String>> {
    validate_name("box name", box_name)?;
    validate_name("key", key)?;
    let mut kv = db
        .kv_box_for(box_name)
        .map_err(|_| anyhow!("persistence_box_open_failed"))?;
    let value = match kv.get_string(key) {
        Ok(cookies) if !cookies.is_empty() => Some(cookies.to_string()),
        Ok(_) | Err(GuiXuError::KeyNotFound(_)) => None,
        Err(_) => return Err(anyhow!("persistence_read_failed")),
    };
    kv.close()
        .map_err(|_| anyhow!("persistence_box_close_failed"))?;
    Ok(value)
}

#[cfg(not(target_arch = "wasm32"))]
fn write_string_to(db: &GuiXu, box_name: &str, key: &str, value: &str) -> Result<()> {
    validate_name("box name", box_name)?;
    validate_name("key", key)?;
    let mut kv = db
        .kv_box_for(box_name)
        .map_err(|_| anyhow!("persistence_box_open_failed"))?;
    kv.put_string(key, value.to_string())
        .map_err(|_| anyhow!("persistence_write_failed"))?;
    kv.close()
        .map_err(|_| anyhow!("persistence_box_close_failed"))?;
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
fn remove_from(db: &GuiXu, box_name: &str, key: &str) -> Result<()> {
    validate_name("box name", box_name)?;
    validate_name("key", key)?;
    let mut kv = db
        .kv_box_for(box_name)
        .map_err(|_| anyhow!("persistence_box_open_failed"))?;
    kv.remove(key)
        .map_err(|_| anyhow!("persistence_remove_failed"))?;
    kv.close()
        .map_err(|_| anyhow!("persistence_box_close_failed"))?;
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
fn clear_box_in(db: &GuiXu, box_name: &str) -> Result<()> {
    validate_name("box name", box_name)?;
    let mut kv = db
        .kv_box_for(box_name)
        .map_err(|_| anyhow!("persistence_box_open_failed"))?;
    kv.clear(true)
        .map_err(|_| anyhow!("persistence_clear_failed"))?;
    kv.close()
        .map_err(|_| anyhow!("persistence_box_close_failed"))?;
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
        if !persistence.persist_session {
            return Ok(false);
        }

        if cookies_json.is_empty() {
            remove_from(&persistence.db, SESSION_BOX, COOKIES_KEY)
                .map_err(|_| anyhow!("persistence_cookie_clear_failed"))?;
        } else {
            write_string_to(&persistence.db, SESSION_BOX, COOKIES_KEY, cookies_json)
                .map_err(|_| anyhow!("persistence_cookie_write_failed"))?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{create_dir_all, remove_dir_all};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn guixu_persistence_reopens_migrates_and_isolates_keychain_sessions() {
        let root = std::env::temp_dir().join(format!(
            "ahutong-sdk-persistence-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock")
                .as_nanos()
        ));
        create_dir_all(&root).expect("temporary persistence root");
        let database = root.join("primary");

        let restored = init(
            database.to_str().expect("utf-8 path"),
            r#"[{"name":"session","value":"seed"}]"#,
            true,
        )
        .expect("initialize persisted session");
        assert!(restored.is_some());
        assert!(put_string("user_cache", "schedule", "cached").expect("write cache"));
        assert_eq!(
            get_string("user_cache", "schedule").expect("read cache"),
            Some("cached".to_string())
        );

        let reopened = init(database.to_str().expect("utf-8 path"), "", true)
            .expect("reopen persisted session");
        assert_eq!(reopened, restored);
        assert_eq!(
            get_string("user_cache", "schedule").expect("read reopened cache"),
            Some("cached".to_string())
        );

        assert!(remove_key("user_cache", "schedule").expect("remove cache"));
        assert_eq!(
            get_string("user_cache", "schedule").expect("read removed cache"),
            None
        );
        assert!(put_string("user_cache", "grade", "cached-grade").expect("write grade"));
        assert!(clear_box("user_cache").expect("clear cache box"));
        assert_eq!(
            get_string("user_cache", "grade").expect("read cleared cache"),
            None
        );

        let keychain_only = root.join("keychain-only");
        let seed = r#"[{"name":"session","value":"keychain"}]"#;
        assert_eq!(
            init(keychain_only.to_str().expect("utf-8 path"), seed, false)
                .expect("initialize keychain-only session"),
            Some(seed.to_string())
        );
        assert!(!save_cookies("must-not-be-written").expect("skip cookie persistence"));
        assert_eq!(
            init(keychain_only.to_str().expect("utf-8 path"), "", false)
                .expect("reopen keychain-only session"),
            None
        );
        let invalid_box = put_string("../escape", "key", "value")
            .expect_err("invalid box name must fail")
            .to_string();
        assert_eq!(invalid_box, "persistence_box_name_invalid");
        assert!(!invalid_box.contains("escape"));

        let invalid_key = put_string("cache", "nested/key", "value")
            .expect_err("invalid key must fail")
            .to_string();
        assert_eq!(invalid_key, "persistence_key_invalid");
        assert!(!invalid_key.contains("nested"));
        let _ = remove_dir_all(root);
    }
}
