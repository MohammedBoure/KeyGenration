use chrono::{SecondsFormat, Utc};
use keygen_common::RuntimeEnvironment;
use native_tls::TlsConnector;
use postgres::config::SslMode;
use postgres::{Client, Config as PostgresConfig, NoTls};
use postgres_native_tls::MakeTlsConnector;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
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

const DEFAULT_LISTEN_ADDRESS: &str = "127.0.0.1:45632";
const BACKEND_MODE: &str = "local-queue-v1";
const RESTAURANT_SECRET: &str = "RestaurantManagement";
const LAB_SECRET: &str = "LabInventoryManagement";
const JEWELRY_SECRET: &str = "JewelryManagement";

type AppResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

#[derive(Clone, Debug)]
struct DatabaseConfig {
    host: String,
    port: u16,
    database: String,
    user: String,
    password: String,
    ssl_mode: String,
    connect_timeout: Duration,
}

impl DatabaseConfig {
    fn from_environment(environment: &RuntimeEnvironment) -> AppResult<Self> {
        Ok(Self {
            host: required_setting(environment, "PGHOST")?,
            port: required_setting(environment, "PGPORT")?.parse()?,
            database: required_setting(environment, "PGDATABASE")?,
            user: required_setting(environment, "PGUSER")?,
            password: required_setting(environment, "PGPASSWORD")?,
            ssl_mode: environment
                .value("PGSSLMODE")
                .unwrap_or_else(|| "require".to_owned()),
            connect_timeout: seconds_from_environment(environment, "PGCONNECT_TIMEOUT", 10),
        })
    }

    fn connect(&self) -> AppResult<Client> {
        let mut config = PostgresConfig::new();
        config
            .host(&self.host)
            .port(self.port)
            .dbname(&self.database)
            .user(&self.user)
            .password(&self.password)
            .connect_timeout(self.connect_timeout);

        match self.ssl_mode.trim().to_ascii_lowercase().as_str() {
            "disable" => {
                config.ssl_mode(SslMode::Disable);
                Ok(config.connect(NoTls)?)
            }
            "require" => {
                config.ssl_mode(SslMode::Require);
                let mut tls = TlsConnector::builder();
                // PostgreSQL sslmode=require encrypts transport without validating its certificate.
                tls.danger_accept_invalid_certs(true);
                Ok(config.connect(MakeTlsConnector::new(tls.build()?))?)
            }
            "verify-full" => {
                config.ssl_mode(SslMode::Require);
                Ok(config.connect(MakeTlsConnector::new(TlsConnector::new()?))?)
            }
            _ => Err("PGSSLMODE must be disable, require, or verify-full.".into()),
        }
    }

    fn read_status(&self) -> AppResult<String> {
        let mut client = self.connect()?;
        let row = client.query_opt("SELECT status FROM server_control WHERE id = 1", &[])?;
        row.map(|record| record.get::<_, String>(0))
            .ok_or_else(|| "The server_control status row is missing.".into())
    }

    fn require_active_installation(&self) -> AppResult<()> {
        match self.read_status()?.trim() {
            "1" => Ok(()),
            "0" => Err("Installation refusee: le generateur est desactive.".into()),
            _ => Err("Installation refusee: etat PostgreSQL invalide.".into()),
        }
    }
}

#[derive(Clone, Debug)]
struct Config {
    database: DatabaseConfig,
    listen_address: String,
    status_interval: Duration,
    upload_interval: Duration,
}

impl Config {
    fn from_environment(environment: &RuntimeEnvironment) -> AppResult<Self> {
        Ok(Self {
            database: DatabaseConfig::from_environment(environment)?,
            listen_address: environment
                .value("KEYGEN_LISTEN_ADDRESS")
                .unwrap_or_else(|| DEFAULT_LISTEN_ADDRESS.to_owned()),
            status_interval: seconds_from_environment(
                environment,
                "KEYGEN_STATUS_INTERVAL_SECONDS",
                15,
            ),
            upload_interval: seconds_from_environment(
                environment,
                "KEYGEN_UPLOAD_INTERVAL_SECONDS",
                5,
            ),
        })
    }
}

#[derive(Clone, Debug)]
struct StoragePaths {
    primary_log: PathBuf,
    cache_log: PathBuf,
    pending_uploads: PathBuf,
    uploaded_log: PathBuf,
    authorization_file: PathBuf,
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
            authorization_file: base_dir.join("AUTHORIZED.txt"),
            maintenance_file: base_dir.join("MAINTENANCE.txt"),
        })
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

#[cfg(windows)]
fn default_primary_log() -> PathBuf {
    default_data_dir().join("generated_keys.txt")
}

#[cfg(not(windows))]
fn default_primary_log() -> PathBuf {
    default_data_dir().join("generated_keys.txt")
}

struct AppState {
    config: Config,
    maintenance: AtomicBool,
    queue_lock: Mutex<()>,
    paths: StoragePaths,
}

impl AppState {
    fn from_environment(environment: &RuntimeEnvironment) -> AppResult<Self> {
        let paths = StoragePaths::from_environment(environment)?;
        let maintenance = paths.maintenance_file.exists();
        Ok(Self {
            config: Config::from_environment(environment)?,
            maintenance: AtomicBool::new(maintenance),
            queue_lock: Mutex::new(()),
            paths,
        })
    }

    fn is_in_maintenance(&self) -> bool {
        self.maintenance.load(Ordering::Relaxed) || self.paths.maintenance_file.exists()
    }

    fn establish_initial_authorization(&self) -> AppResult<()> {
        if self.paths.authorization_file.exists() {
            return Ok(());
        }
        self.config.database.require_active_installation()?;
        self.apply_remote_status("1")?;
        fs::write(
            &self.paths.authorization_file,
            format!("STATUS=\"1\" - AUTHORIZED at {}\n", now_timestamp()),
        )?;
        Ok(())
    }

    fn apply_remote_status(&self, status: &str) -> AppResult<()> {
        match status.trim() {
            "0" => {
                self.maintenance.store(true, Ordering::Relaxed);
                fs::write(
                    &self.paths.maintenance_file,
                    format!("STATUS=\"0\" - MAINTENANCE MODE at {}\n", now_timestamp()),
                )?;
                println!("[MAINTENANCE] Key generation disabled by PostgreSQL status.");
            }
            "1" => {
                self.maintenance.store(false, Ordering::Relaxed);
                if self.paths.maintenance_file.exists() {
                    fs::remove_file(&self.paths.maintenance_file)?;
                }
                println!("[MAINTENANCE] Key generation enabled by PostgreSQL status.");
            }
            _ => return Err("Invalid server_control status in PostgreSQL.".into()),
        }
        Ok(())
    }

    fn check_remote_status(&self) -> AppResult<()> {
        self.apply_remote_status(&self.config.database.read_status()?)
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
        let timestamp = now_timestamp();
        pending.push(PendingUpload {
            sync_id: format!("{timestamp}:{request_code}:{activation_key}"),
            request_code: request_code.to_owned(),
            activation_key: activation_key.to_owned(),
            timestamp,
            ip: client_ip.to_owned(),
        });
        save_pending_uploads(&self.paths.pending_uploads, &pending)?;
        Ok(())
    }

    fn upload_pending(&self) -> AppResult<()> {
        let pending = {
            let _guard = lock_mutex(&self.queue_lock);
            load_pending_uploads(&self.paths.pending_uploads)?
        };
        if pending.is_empty() {
            return Ok(());
        }

        let mut client = self.config.database.connect()?;
        let mut transaction = client.transaction()?;
        for record in &pending {
            let sync_id = record.upload_id();
            transaction.execute(
                r#"
                INSERT INTO activation_logs
                    (sync_id, request_code, activation_key, generated_at, device_ip)
                VALUES ($1, $2, $3, $4, $5)
                ON CONFLICT (sync_id) DO NOTHING
                "#,
                &[
                    &sync_id,
                    &record.request_code,
                    &record.activation_key,
                    &record.timestamp,
                    &record.ip,
                ],
            )?;
        }
        transaction.commit()?;

        let _guard = lock_mutex(&self.queue_lock);
        let queued = load_pending_uploads(&self.paths.pending_uploads)?;
        if queued.len() < pending.len() || queued[..pending.len()] != pending {
            return Err("Pending upload queue changed unexpectedly.".into());
        }
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
        save_pending_uploads(&self.paths.pending_uploads, &queued[pending.len()..])?;
        println!("[POSTGRESQL] Uploaded {} queued records.", pending.len());
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct GenerateRequest {
    request_code: Option<String>,
    app_type: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
struct PendingUpload {
    #[serde(default)]
    sync_id: String,
    request_code: String,
    activation_key: String,
    timestamp: String,
    ip: String,
}

impl PendingUpload {
    fn upload_id(&self) -> String {
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

fn main() -> AppResult<()> {
    let environment = RuntimeEnvironment::load()?;
    if env::args().any(|argument| argument == "--authorize-install") {
        DatabaseConfig::from_environment(&environment)?.require_active_installation()?;
        println!("Installation authorized by PostgreSQL status.");
        return Ok(());
    }

    let state = Arc::new(AppState::from_environment(&environment)?);
    state.establish_initial_authorization()?;
    spawn_remote_controller(Arc::clone(&state));
    spawn_uploader(Arc::clone(&state));

    let server = Server::http(&state.config.listen_address)?;
    println!("--- KeyGenService Rust backend ---");
    println!(
        "Local API: http://{}/generate_key",
        state.config.listen_address
    );
    println!("Pending queue: {}", state.paths.pending_uploads.display());
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
                eprintln!("[STATUS] {error}");
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
                "backend_mode": BACKEND_MODE,
                "authorized": state.paths.authorization_file.exists(),
                "maintenance": state.is_in_maintenance(),
                "pending_uploads": load_pending_uploads(&state.paths.pending_uploads)
                    .map(|records| records.len())
                    .unwrap_or_default()
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
                "error": "Server is under maintenance. Key generation is disabled.",
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
    let Some(activation_key) = generate_activation_key(&request_code, app_type) else {
        return (400, json!({"error": "Invalid app_type."}));
    };
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

fn generate_activation_key(request_code: &str, app_type: &str) -> Option<String> {
    let secret = match app_type {
        "Restaurant" => RESTAURANT_SECRET,
        "Lab" => LAB_SECRET,
        "Jewelry" => JEWELRY_SECRET,
        _ => return None,
    };
    let digest = Sha256::digest(format!("{request_code}::{secret}").as_bytes());
    let encoded = digest
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect::<String>();
    let key = &encoded[..16];
    Some(
        (0..16)
            .step_by(4)
            .map(|start| &key[start..start + 4])
            .collect::<Vec<_>>()
            .join("-"),
    )
}

fn valid_request_code(request_code: &str) -> bool {
    request_code.len() == 14
        && request_code.as_bytes()[4] == b'-'
        && request_code.as_bytes()[9] == b'-'
}

fn required_setting(environment: &RuntimeEnvironment, name: &str) -> AppResult<String> {
    environment
        .value(name)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("{name} is required for PostgreSQL synchronization.").into())
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
    Duration::from_secs(
        environment
            .value(variable)
            .and_then(|value| value.parse().ok())
            .unwrap_or(default),
    )
}

fn lock_mutex<T>(lock: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_expected_keys_locally_for_supported_products() {
        assert_eq!(
            generate_activation_key("F81A-67A7-C6AA", "Restaurant"),
            Some("EE8C-551F-0A90-73F5".to_owned())
        );
        assert_eq!(
            generate_activation_key("F81A-67A7-C6AA", "Lab"),
            Some("F74D-9047-9117-B8AF".to_owned())
        );
        assert_eq!(
            generate_activation_key("F81A-67A7-C6AA", "Jewelry"),
            Some("EF4E-047E-2AF5-63B7".to_owned())
        );
        assert!(generate_activation_key("F81A-67A7-C6AA", "Unknown").is_none());
    }

    #[test]
    fn validates_request_format() {
        assert!(valid_request_code("F81A-67A7-C6AA"));
        assert!(!valid_request_code("F81A67A7C6AA"));
    }
}
