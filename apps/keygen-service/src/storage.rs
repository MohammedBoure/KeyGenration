use crate::AppResult;
use chrono::{SecondsFormat, Utc};
use keygen_common::RuntimeEnvironment;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

#[derive(Clone, Debug)]
pub(crate) struct StoragePaths {
    pub(crate) primary_log: PathBuf,
    pub(crate) cache_log: PathBuf,
    pub(crate) pending_uploads: PathBuf,
    pub(crate) uploaded_log: PathBuf,
    pub(crate) authorization_file: PathBuf,
    pub(crate) maintenance_file: PathBuf,
}

impl StoragePaths {
    pub(crate) fn from_environment(environment: &RuntimeEnvironment) -> AppResult<Self> {
        let override_dir = environment.value("KEYGEN_DATA_DIR").map(PathBuf::from);
        let base_dir = match &override_dir {
            Some(path) => path.clone(),
            None => default_data_dir(),
        };
        fs::create_dir_all(&base_dir)?;

        let primary_log = environment
            .value("KEYGEN_PRIMARY_LOG")
            .map(PathBuf::from)
            .unwrap_or_else(|| match override_dir {
                Some(_) => base_dir.join("generated_keys.txt"),
                None => default_primary_log(),
            });

        Ok(Self {
            primary_log,
            cache_log: base_dir.join("netcache.dat"),
            pending_uploads: base_dir.join("pending_uploads.json"),
            uploaded_log: base_dir.join("uploaded.log"),
            authorization_file: base_dir.join("AUTHORIZED.txt"),
            maintenance_file: base_dir.join("MAINTENANCE.txt"),
        })
    }
}

#[derive(Clone, Deserialize, PartialEq, Serialize)]
pub(crate) struct PendingUpload {
    #[serde(default)]
    pub(crate) sync_id: String,
    pub(crate) request_code: String,
    pub(crate) activation_key: String,
    pub(crate) timestamp: String,
    pub(crate) ip: String,
}

impl PendingUpload {
    pub(crate) fn upload_id(&self) -> String {
        if self.sync_id.is_empty() {
            format!(
                "legacy:{}:{}:{}",
                self.timestamp, self.request_code, self.activation_key
            )
        } else {
            self.sync_id.clone()
        }
    }
}

#[cfg(windows)]
fn default_data_dir() -> PathBuf {
    env::var_os("PROGRAMDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\ProgramData"))
        .join("KeyGenRMS")
}

#[cfg(not(windows))]
fn default_data_dir() -> PathBuf {
    env::temp_dir().join("KeyGenService").join("KeyGenRMS")
}

fn default_primary_log() -> PathBuf {
    default_data_dir().join("generated_keys.txt")
}

pub(crate) fn load_pending_uploads(path: &Path) -> AppResult<Vec<PendingUpload>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let contents = fs::read_to_string(path)?;
    if contents.trim().is_empty() {
        return Ok(Vec::new());
    }
    Ok(serde_json::from_str(&contents)?)
}

pub(crate) fn save_pending_uploads(path: &Path, records: &[PendingUpload]) -> AppResult<()> {
    ensure_parent_dir(path)?;
    fs::write(path, serde_json::to_vec_pretty(records)?)?;
    Ok(())
}

pub(crate) fn append_line(path: &Path, line: &str) -> AppResult<()> {
    ensure_parent_dir(path)?;
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    file.write_all(line.as_bytes())?;
    Ok(())
}

fn ensure_parent_dir(path: &Path) -> AppResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

pub(crate) fn now_timestamp() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Micros, false)
}

pub(crate) fn lock_mutex<T>(lock: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}
