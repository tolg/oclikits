use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use serde::{Deserialize, Serialize};
use tokio::process::Command;

use crate::config::config_dir;
use crate::error::{OcError, Result};
use crate::i18n::tr;

fn default_version() -> u32 {
    1
}

/// Contents of `~/.config/oclikits/sessions.json`.
///
/// oclikits only stores the mapping from a logical context to an OpenCode
/// session ID. Message history always belongs to OpenCode.
#[derive(Debug, Serialize, Deserialize)]
struct SessionsFile {
    #[serde(default = "default_version")]
    version: u32,
    #[serde(default)]
    ocq: Option<SessionEntry>,
    #[serde(default)]
    ocsh: BTreeMap<String, SessionEntry>,
}

impl Default for SessionsFile {
    fn default() -> Self {
        Self {
            version: default_version(),
            ocq: None,
            ocsh: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SessionEntry {
    session_id: String,
}

impl SessionEntry {
    fn new(session_id: &str) -> Self {
        Self {
            session_id: session_id.to_string(),
        }
    }
}

/// Persistent mapping between oclikits contexts and OpenCode sessions.
pub struct SessionStore {
    path: PathBuf,
}

impl SessionStore {
    pub fn new() -> Self {
        Self::at(config_dir().join("sessions.json"))
    }

    pub fn at(path: PathBuf) -> Self {
        Self { path }
    }

    #[allow(dead_code)]
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn get_ocq(&self) -> Result<Option<String>> {
        self.with_lock(|data| Ok(data.ocq.as_ref().map(|entry| entry.session_id.clone())))
    }

    pub fn set_ocq(&self, session_id: &str) -> Result<()> {
        self.with_lock(|data| {
            data.ocq = Some(SessionEntry::new(session_id));
            self.save(data)
        })
    }

    pub fn clear_ocq(&self) -> Result<()> {
        self.with_lock(|data| {
            data.ocq = None;
            self.save(data)
        })
    }

    pub fn get_ocsh(&self, dir: &Path) -> Result<Option<String>> {
        let key = canonical_key(dir);
        self.with_lock(|data| Ok(data.ocsh.get(&key).map(|entry| entry.session_id.clone())))
    }

    pub fn set_ocsh(&self, dir: &Path, session_id: &str) -> Result<()> {
        let key = canonical_key(dir);
        self.with_lock(|data| {
            data.ocsh.insert(key.clone(), SessionEntry::new(session_id));
            self.save(data)
        })
    }

    pub fn clear_ocsh(&self, dir: &Path) -> Result<()> {
        let key = canonical_key(dir);
        self.with_lock(|data| {
            data.ocsh.remove(&key);
            self.save(data)
        })
    }

    /// Lock -> read -> mutate. The write happens inside the callback through
    /// [`SessionStore::save`], so a read-modify-write cycle is atomic.
    fn with_lock<T>(&self, action: impl FnOnce(&mut SessionsFile) -> Result<T>) -> Result<T> {
        let lock_path = self.lock_path()?;
        let lock_file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(lock_path)?;
        lock_exclusive(&lock_file)?;

        let mut data = self.read();
        action(&mut data)
        // The lock is released when `lock_file` goes out of scope.
    }

    fn read(&self) -> SessionsFile {
        match fs::read_to_string(&self.path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
            Err(_) => SessionsFile::default(),
        }
    }

    /// Write through a temporary file followed by an atomic rename.
    fn save(&self, data: &SessionsFile) -> Result<()> {
        let parent = self
            .path
            .parent()
            .ok_or_else(|| OcError::Message("invalid sessions.json path".to_string()))?;
        fs::create_dir_all(parent)?;

        let tmp = parent.join("sessions.json.tmp");
        let body = serde_json::to_string_pretty(data)?;
        {
            let mut file = File::create(&tmp)?;
            file.write_all(body.as_bytes())?;
            file.write_all(b"\n")?;
            file.sync_all()?;
        }
        fs::rename(&tmp, &self.path)?;
        Ok(())
    }

    fn lock_path(&self) -> Result<PathBuf> {
        let parent = self
            .path
            .parent()
            .ok_or_else(|| OcError::Message("invalid sessions.json path".to_string()))?;
        fs::create_dir_all(parent)?;
        Ok(parent.join("sessions.json.lock"))
    }
}

impl Default for SessionStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Canonical key used for per-directory `ocsh` sessions.
pub fn canonical_key(dir: &Path) -> String {
    let absolute = if dir.is_absolute() {
        dir.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(dir)
    };

    fs::canonicalize(&absolute)
        .unwrap_or(absolute)
        .to_string_lossy()
        .into_owned()
}

/// Delete an OpenCode session.
///
/// A session that no longer exists counts as already deleted.
pub async fn delete_opencode_session(session_id: &str) -> Result<()> {
    let output = Command::new("opencode")
        .args(["session", "delete", session_id])
        .stdin(Stdio::null())
        .output()
        .await
        .map_err(|err| match err.kind() {
            std::io::ErrorKind::NotFound => OcError::OpencodeNotFound,
            _ => OcError::Io(err),
        })?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    if stderr.to_lowercase().contains("not found") || stdout.to_lowercase().contains("not found") {
        return Ok(());
    }

    let detail = stderr.trim();
    let zh = format!("删除 OpenCode 会话 {} 失败：{}", session_id, detail);
    let en = format!(
        "Failed to delete OpenCode session {}: {}",
        session_id, detail
    );
    Err(OcError::Message(tr(&zh, &en).to_string()))
}

#[cfg(unix)]
fn lock_exclusive(file: &File) -> Result<()> {
    use std::os::unix::io::AsRawFd;

    let rc = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX) };
    if rc == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error().into())
    }
}

#[cfg(not(unix))]
fn lock_exclusive(_file: &File) -> Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    fn temp_store() -> (SessionStore, PathBuf) {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "oclikits-test-{}-{}-{}",
            std::process::id(),
            nanos,
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        (SessionStore::at(dir.join("sessions.json")), dir)
    }

    #[test]
    fn ocq_roundtrip() {
        let (store, dir) = temp_store();

        assert_eq!(store.get_ocq().unwrap(), None);
        store.set_ocq("ses_a").unwrap();
        assert_eq!(store.get_ocq().unwrap(), Some("ses_a".to_string()));
        store.clear_ocq().unwrap();
        assert_eq!(store.get_ocq().unwrap(), None);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn ocsh_is_scoped_per_directory() {
        let (store, dir) = temp_store();
        let a = dir.join("a");
        let b = dir.join("b");
        fs::create_dir_all(&a).unwrap();
        fs::create_dir_all(&b).unwrap();

        store.set_ocsh(&a, "ses_a").unwrap();
        store.set_ocsh(&b, "ses_b").unwrap();

        assert_eq!(store.get_ocsh(&a).unwrap(), Some("ses_a".to_string()));
        assert_eq!(store.get_ocsh(&b).unwrap(), Some("ses_b".to_string()));

        store.clear_ocsh(&a).unwrap();
        assert_eq!(store.get_ocsh(&a).unwrap(), None);
        assert_eq!(store.get_ocsh(&b).unwrap(), Some("ses_b".to_string()));

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn canonical_key_normalizes_paths() {
        let (_, dir) = temp_store();
        fs::create_dir_all(dir.join("sub")).unwrap();

        let plain = canonical_key(&dir);
        let dotted = canonical_key(&dir.join("."));
        let relative = canonical_key(&dir.join("sub").join(".."));

        assert_eq!(plain, dotted);
        assert_eq!(plain, relative);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn stores_version_and_valid_json() {
        let (store, dir) = temp_store();
        store.set_ocq("ses_a").unwrap();

        let content = fs::read_to_string(store.path()).unwrap();
        let value: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(value["version"], 1);
        assert_eq!(value["ocq"]["session_id"], "ses_a");

        let _ = fs::remove_dir_all(dir);
    }
}
