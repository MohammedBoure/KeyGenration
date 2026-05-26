use chrono::{SecondsFormat, Utc};
use keygen_common::RuntimeEnvironment;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::env;
use std::error::Error;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};

const DEFAULT_CLOUD_API_URL: &str = "https://activation.example.com";
const DEFAULT_LISTEN_ADDRESS: &str = "127.0.0.1:45632";

type AppResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

#[derive(Clone, Debug)]
struct Config {
    listen_address: String,
    api_token: String,
    status_interval: Duration,
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
        }
    }
}

#[derive(Clone, Debug)]
struct StoragePaths {
    primary_log: PathBuf,
    cache_log: PathBuf,
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
    cloud_api_url: String,
    maintenance: AtomicBool,
    log_lock: Mutex<()>,
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
            cloud_api_url,
            maintenance: AtomicBool::new(maintenance),
            log_lock: Mutex::new(()),
            paths,
        })
    }

    fn cloud_api_url(&self) -> String {
        self.cloud_api_url.clone()
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

    fn save_generated_key(&self, request_code: &str, activation_key: &str) -> AppResult<()> {
        if self.is_in_maintenance() {
            return Err("Maintenance mode: key generation is disabled.".into());
        }

        let _guard = lock_mutex(&self.log_lock);
        let line = format!("REQUEST_CODE: {request_code}, ACTIVATION_KEY: {activation_key}\n");
        append_line(&self.paths.primary_log, &line)?;
        append_line(&self.paths.cache_log, &line)?;
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

    fn request_activation_key(&self, request_code: &str, app_type: &str) -> AppResult<String> {
        if self.config.api_token.is_empty() {
            return Err("KEYGEN_API_SECRET_TOKEN is not set; generation disabled.".into());
        }

        let url = format!("{}/api/v1/generate-key", self.cloud_api_url());
        let response = ureq::post(&url)
            .set(
                "Authorization",
                &format!("Bearer {}", self.config.api_token),
            )
            .set("Content-Type", "application/json")
            .timeout(Duration::from_secs(12))
            .send_json(json!(CloudGenerateRequest {
                request_code,
                app_type,
            }))?;
        let generated: CloudGenerateResponse = response.into_json()?;
        Ok(generated.activation_key)
    }
}

#[derive(Debug, Deserialize)]
struct GenerateRequest {
    request_code: Option<String>,
    app_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ServerStatus {
    status: String,
}

#[derive(Serialize)]
struct CloudGenerateRequest<'a> {
    request_code: &'a str,
    app_type: &'a str,
}

#[derive(Deserialize)]
struct CloudGenerateResponse {
    activation_key: String,
}

fn main() -> AppResult<()> {
    let environment = RuntimeEnvironment::load()?;
    let state = Arc::new(AppState::from_environment(&environment)?);

    if state.config.api_token.is_empty() {
        eprintln!("[CONFIG] KEYGEN_API_SECRET_TOKEN is unset; remote generation is disabled.");
    }

    state.check_remote_status()?;
    spawn_remote_controller(Arc::clone(&state));

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
    if let Err(error) = state.check_remote_status() {
        eprintln!("[AUTHORIZATION] {error}");
        return (
            503,
            json!({"error": "Remote authorization unavailable; key generation denied."}),
        );
    }
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
    let activation_key = match state.request_activation_key(&request_code, app_type) {
        Ok(key) => key,
        Err(error) => {
            eprintln!("[GENERATE CLOUD] {error}");
            return (
                503,
                json!({"error": "Cloud generation denied or unavailable."}),
            );
        }
    };

    if let Err(error) = state.save_generated_key(&request_code, &activation_key) {
        eprintln!("[GENERATE] {error}");
        return (503, json!({"error": "Key generation failed."}));
    }

    (
        200,
        json!({
            "request_code": request_code,
            "activation_key": activation_key,
            "app_type": app_type,
            "status": "generated_by_cloud"
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

fn lock_mutex<T>(lock: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

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
