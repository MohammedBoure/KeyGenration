use chrono::{SecondsFormat, Utc};
use keygen_common::RuntimeEnvironment;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::env;
use std::error::Error;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time::Duration;
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};

const DEFAULT_CLOUD_API_URL: &str = "https://activation.example.com";
const DEFAULT_LISTEN_ADDRESS: &str = "127.0.0.1:45632";
const RESTAURANT_SECRET: &str = "RestaurantManagement";
const LAB_SECRET: &str = "LabInventoryManagement";
const JEWELRY_SECRET: &str = "JewelryManagement";

type AppResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

#[derive(Clone, Debug)]
struct Config {
    listen_address: String,
    api_token: String,
    status_interval: Duration,
    upload_interval: Duration,
}

impl Config {
    fn from_environment(environment: &RuntimeEnvironment) -> Self {
        Self {
            listen_address: environment
                .value("KEYGEN_LISTEN_ADDRESS")
                .unwrap_or_else(|| DEFAULT_LISTEN_ADDRESS.to_owned()),
            api_token: environment
                .value("KEYGEN_API_SECRET_TOKEN")
                .unwrap_or_default(),
            status_interval: seconds_from_environment(
                environment,
                "KEYGEN_STATUS_INTERVAL_SECONDS",
                300,
            ),
            upload_interval: seconds_from_environment(
                environment,
                "KEYGEN_UPLOAD_INTERVAL_SECONDS",
                5,
            ),
        }
    }
}

#[derive(Clone, Debug)]
struct StoragePaths {
    primary_log: PathBuf,
    cache_log: PathBuf,
    pending_uploads: PathBuf,
    uploaded_log: PathBuf,
    maintenance_file: PathBuf,
}

impl StoragePaths {
    fn from_environment(environment: &RuntimeEnvironment) -> AppResult<Self> {
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
            maintenance_file: base_dir.join("MAINTENANCE.txt"),
        })
    }
}

#[cfg(windows)]
fn default_data_dir() -> PathBuf {
    env::var_os("PROGRAMDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\ProgramData"))
        .join("SystemLogs")
}

#[cfg(not(windows))]
fn default_data_dir() -> PathBuf {
    env::temp_dir().join("KeyGenService").join("SystemLogs")
}

#[cfg(windows)]
fn default_primary_log() -> PathBuf {
    PathBuf::from(r"C:\key_storage\generated_keys.txt")
}

#[cfg(not(windows))]
fn default_primary_log() -> PathBuf {
    default_data_dir().join("generated_keys.txt")
}

struct AppState {
    config: Config,
    cloud_api_url: RwLock<String>,
    maintenance: AtomicBool,
    queue_lock: Mutex<()>,
    paths: StoragePaths,
}

impl AppState {
    fn from_environment(environment: &RuntimeEnvironment) -> AppResult<Self> {
        let cloud_api_url = environment
            .value("KEYGEN_CLOUD_API_URL")
            .unwrap_or_else(|| DEFAULT_CLOUD_API_URL.to_owned());
        let cloud_api_url = normalize_cloud_url(&cloud_api_url)
            .ok_or("KEYGEN_CLOUD_API_URL must begin with http:// or https://")?;
        let paths = StoragePaths::from_environment(environment)?;
        let maintenance = paths.maintenance_file.exists();

        Ok(Self {
            config: Config::from_environment(environment),
            cloud_api_url: RwLock::new(cloud_api_url),
            maintenance: AtomicBool::new(maintenance),
            queue_lock: Mutex::new(()),
            paths,
        })
    }

    fn cloud_api_url(&self) -> String {
        lock_read(&self.cloud_api_url).clone()
    }

    fn update_cloud_api_url(&self, url: &str) -> Result<(), &'static str> {
        let normalized =
            normalize_cloud_url(url).ok_or("server_url must begin with http:// or https://")?;
        *lock_write(&self.cloud_api_url) = normalized;
        Ok(())
    }

    fn is_in_maintenance(&self) -> bool {
        self.maintenance.load(Ordering::Relaxed) || self.paths.maintenance_file.exists()
    }

    fn apply_remote_status(&self, status: &str) -> AppResult<()> {
        match status {
            "0" => {
                self.maintenance.store(true, Ordering::Relaxed);
                fs::write(
                    &self.paths.maintenance_file,
                    format!("STATUS=\"0\" - MAINTENANCE MODE at {}\n", now_timestamp()),
                )?;
                println!("[MAINTENANCE] Key generation disabled by cloud API.");
            }
            "1" => {
                self.maintenance.store(false, Ordering::Relaxed);
                if self.paths.maintenance_file.exists() {
                    fs::remove_file(&self.paths.maintenance_file)?;
                }
                println!("[MAINTENANCE] Key generation enabled by cloud API.");
            }
            _ => eprintln!("[REMOTE CHECK] Ignored unknown status: {status}"),
        }

        Ok(())
    }

    fn save_generated_key(
        &self,
        request_code: &str,
        activation_key: &str,
        client_ip: &str,
    ) -> AppResult<()> {
        if self.is_in_maintenance() {
            return Err("Maintenance mode: key generation is disabled.".into());
        }

        let _guard = lock_mutex(&self.queue_lock);
        let line = format!("REQUEST_CODE: {request_code}, ACTIVATION_KEY: {activation_key}\n");
        append_line(&self.paths.primary_log, &line)?;
        append_line(&self.paths.cache_log, &line)?;

        let mut pending = load_pending_uploads(&self.paths.pending_uploads)?;
        pending.push(PendingUpload {
            request_code: request_code.to_owned(),
            activation_key: activation_key.to_owned(),
            timestamp: now_timestamp(),
            ip: client_ip.to_owned(),
        });
        save_pending_uploads(&self.paths.pending_uploads, &pending)?;
        Ok(())
    }

    fn check_remote_status(&self) -> AppResult<()> {
        if self.config.api_token.is_empty() {
            return Err("KEYGEN_API_SECRET_TOKEN is not set; remote control disabled.".into());
        }

        let url = format!("{}/api/v1/server-status", self.cloud_api_url());
        let response = ureq::get(&url)
            .set(
                "Authorization",
                &format!("Bearer {}", self.config.api_token),
            )
            .set("Content-Type", "application/json")
            .timeout(Duration::from_secs(12))
            .call()?;
        let status: ServerStatus = response.into_json()?;
        self.apply_remote_status(status.status.trim())
    }

    fn upload_pending(&self) -> AppResult<()> {
        if self.is_in_maintenance() || self.config.api_token.is_empty() {
            return Ok(());
        }

        let _guard = lock_mutex(&self.queue_lock);
        let pending = load_pending_uploads(&self.paths.pending_uploads)?;
        if pending.is_empty() {
            return Ok(());
        }

        let records: Vec<CloudActivationLog> = pending
            .iter()
            .map(|record| CloudActivationLog {
                request_code: &record.request_code,
                activation_key: &record.activation_key,
                generated_at: &record.timestamp,
                device_ip: &record.ip,
            })
            .collect();
        let url = format!("{}/api/v1/activation-logs", self.cloud_api_url());

        ureq::post(&url)
            .set(
                "Authorization",
                &format!("Bearer {}", self.config.api_token),
            )
            .set("Content-Type", "application/json")
            .timeout(Duration::from_secs(10))
            .send_json(&records)?;

        for record in &pending {
            append_line(
                &self.paths.uploaded_log,
                &format!(
                    "{} | {} | {}\n",
                    now_timestamp(),
                    record.request_code,
                    record.activation_key
                ),
            )?;
        }
        save_pending_uploads(&self.paths.pending_uploads, &[])?;
        println!("[Cloud API] Uploaded {} queued records.", pending.len());
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct GenerateRequest {
    request_code: Option<String>,
    app_type: Option<String>,
    server_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ServerStatus {
    status: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct PendingUpload {
    request_code: String,
    activation_key: String,
    timestamp: String,
    ip: String,
}

#[derive(Serialize)]
struct CloudActivationLog<'a> {
    request_code: &'a str,
    activation_key: &'a str,
    generated_at: &'a str,
    device_ip: &'a str,
}

fn main() -> AppResult<()> {
    let environment = RuntimeEnvironment::load()?;
    let state = Arc::new(AppState::from_environment(&environment)?);

    if state.config.api_token.is_empty() {
        eprintln!(
            "[CONFIG] KEYGEN_API_SECRET_TOKEN is unset; queued records cannot upload until configured."
        );
    }

    spawn_remote_controller(Arc::clone(&state));
    spawn_uploader(Arc::clone(&state));

    let server = Server::http(&state.config.listen_address)?;
    println!("--- KeyGenService Rust backend ---");
    println!(
        "Local API: http://{}/generate_key",
        state.config.listen_address
    );
    println!("Cloud API: {}", state.cloud_api_url());
    if environment.exists() {
        println!("Config: {}", environment.path().display());
    }
    println!("Queue: {}", state.paths.pending_uploads.display());

    for request in server.incoming_requests() {
        handle_request(request, &state);
    }

    Ok(())
}

fn spawn_remote_controller(state: Arc<AppState>) {
    thread::spawn(move || {
        loop {
            if let Err(error) = state.check_remote_status() {
                eprintln!("[REMOTE CHECK] {error}");
            }
            thread::sleep(state.config.status_interval);
        }
    });
}

fn spawn_uploader(state: Arc<AppState>) {
    thread::spawn(move || {
        loop {
            if let Err(error) = state.upload_pending() {
                eprintln!("[UPLOAD] {error}");
            }
            thread::sleep(state.config.upload_interval);
        }
    });
}

fn handle_request(mut request: Request, state: &Arc<AppState>) {
    match (request.method(), request.url()) {
        (&Method::Post, "/generate_key") => {
            let response = generate_key_response(&mut request, state);
            respond_json(request, response.0, &response.1);
        }
        (&Method::Get, "/health") => respond_json(
            request,
            200,
            &json!({
                "status": "ok",
                "maintenance": state.is_in_maintenance(),
                "cloud_api_url": state.cloud_api_url()
            }),
        ),
        _ => respond_json(request, 404, &json!({"error": "Not found"})),
    }
}

fn generate_key_response(request: &mut Request, state: &Arc<AppState>) -> (u16, Value) {
    if state.is_in_maintenance() {
        return (
            503,
            json!({
                "error": "Server is under maintenance. Key generation is temporarily disabled.",
                "status": "maintenance_mode"
            }),
        );
    }

    let mut body = String::new();
    if request.as_reader().read_to_string(&mut body).is_err() {
        return (400, json!({"error": "Unable to read JSON body"}));
    }
    let payload: GenerateRequest = match serde_json::from_str(&body) {
        Ok(payload) => payload,
        Err(_) => return (400, json!({"error": "Invalid JSON body"})),
    };

    if let Some(server_url) = payload.server_url.as_deref()
        && let Err(error) = state.update_cloud_api_url(server_url)
    {
        return (400, json!({"error": error}));
    }

    let Some(raw_code) = payload.request_code else {
        return (400, json!({"error": "Missing 'request_code' in JSON body"}));
    };
    let request_code = raw_code.trim().to_ascii_uppercase();
    if !valid_request_code(&request_code) {
        return (
            400,
            json!({"error": "Invalid request_code format. Expected 'XXXX-XXXX-XXXX'."}),
        );
    }

    let app_type = payload.app_type.as_deref().unwrap_or("Restaurant");
    let activation_key = generate_activation_key(&request_code, app_type);
    let client_ip = request
        .remote_addr()
        .map(|address| address.ip().to_string())
        .unwrap_or_else(|| "unknown".to_owned());

    if let Err(error) = state.save_generated_key(&request_code, &activation_key, &client_ip) {
        eprintln!("[GENERATE] {error}");
        return (503, json!({"error": "Key generation failed."}));
    }

    (
        200,
        json!({
            "request_code": request_code,
            "activation_key": activation_key,
            "app_type": app_type,
            "status": "generated_and_queued"
        }),
    )
}

fn respond_json(request: Request, status_code: u16, body: &Value) {
    let content_type =
        Header::from_bytes("Content-Type", "application/json; charset=utf-8").unwrap();
    let response = Response::from_string(body.to_string())
        .with_status_code(StatusCode(status_code))
        .with_header(content_type);
    if let Err(error) = request.respond(response) {
        eprintln!("[HTTP] Failed to send response: {error}");
    }
}

fn generate_activation_key(request_code: &str, app_type: &str) -> String {
    let secret = match app_type {
        "Lab" => LAB_SECRET,
        "Jewelry" => JEWELRY_SECRET,
        _ => RESTAURANT_SECRET,
    };
    let digest = Sha256::digest(format!("{request_code}::{secret}").as_bytes());
    let encoded = format!("{digest:X}");
    let key = &encoded[..16];
    (0..16)
        .step_by(4)
        .map(|start| &key[start..start + 4])
        .collect::<Vec<_>>()
        .join("-")
}

fn valid_request_code(request_code: &str) -> bool {
    request_code.len() == 14
        && request_code.as_bytes()[4] == b'-'
        && request_code.as_bytes()[9] == b'-'
}

fn normalize_cloud_url(url: &str) -> Option<String> {
    let trimmed = url.trim().trim_end_matches('/');
    if (trimmed.starts_with("http://") || trimmed.starts_with("https://"))
        && !trimmed.chars().any(char::is_whitespace)
    {
        Some(trimmed.to_owned())
    } else {
        None
    }
}

fn load_pending_uploads(path: &Path) -> AppResult<Vec<PendingUpload>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let contents = fs::read_to_string(path)?;
    if contents.trim().is_empty() {
        return Ok(Vec::new());
    }
    Ok(serde_json::from_str(&contents)?)
}

fn save_pending_uploads(path: &Path, records: &[PendingUpload]) -> AppResult<()> {
    ensure_parent_dir(path)?;
    fs::write(path, serde_json::to_vec_pretty(records)?)?;
    Ok(())
}

fn append_line(path: &Path, line: &str) -> AppResult<()> {
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

fn now_timestamp() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Micros, false)
}

fn seconds_from_environment(
    environment: &RuntimeEnvironment,
    variable: &str,
    default: u64,
) -> Duration {
    let seconds = environment
        .value(variable)
        .and_then(|value| value.parse().ok())
        .unwrap_or(default);
    Duration::from_secs(seconds)
}

fn lock_read<T>(lock: &RwLock<T>) -> std::sync::RwLockReadGuard<'_, T> {
    lock.read().unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn lock_write<T>(lock: &RwLock<T>) -> std::sync::RwLockWriteGuard<'_, T> {
    lock.write()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn lock_mutex<T>(lock: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_existing_restaurant_key_algorithm() {
        assert_eq!(
            generate_activation_key("F81A-67A7-C6AA", "Restaurant"),
            "EE8C-551F-0A90-73F5"
        );
    }

    #[test]
    fn supports_application_specific_secrets() {
        let restaurant = generate_activation_key("F81A-67A7-C6AA", "Restaurant");
        let lab = generate_activation_key("F81A-67A7-C6AA", "Lab");
        let jewelry = generate_activation_key("F81A-67A7-C6AA", "Jewelry");
        assert_ne!(restaurant, lab);
        assert_ne!(restaurant, jewelry);
        assert_ne!(lab, jewelry);
    }

    #[test]
    fn validates_request_format_compatible_with_python_service() {
        assert!(valid_request_code("F81A-67A7-C6AA"));
        assert!(!valid_request_code("F81A67A7C6AA"));
    }

    #[test]
    fn validates_cloud_url_scheme() {
        assert_eq!(
            normalize_cloud_url("https://example.test/api/"),
            Some("https://example.test/api".to_owned())
        );
        assert_eq!(normalize_cloud_url("file:///tmp/api"), None);
    }
}
