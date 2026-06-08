mod database;
mod generation;
mod http;
mod storage;

use database::DatabaseConfig;
use generation::{generate_activation_key, valid_request_code};
use http::handle_request;
use keygen_common::{GenerationToken, RuntimeEnvironment};
use std::env;
use std::error::Error;
use std::fs;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use storage::{
    PendingUpload, StoragePaths, append_line, load_pending_uploads, lock_mutex, now_timestamp,
    save_pending_uploads,
};
use tiny_http::Server;

const DEFAULT_LISTEN_ADDRESS: &str = "127.0.0.1:45632";
const DEV_GENERATE_KEY_FLAG: &str = "--dev-generate-key";

type AppResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

#[derive(Clone)]
pub(crate) struct Config {
    database: DatabaseConfig,
    pub(crate) tokens: Vec<GenerationToken>,
    listen_address: String,
    status_interval: Duration,
    upload_interval: Duration,
}

impl Config {
    fn from_environment(environment: &RuntimeEnvironment) -> AppResult<Self> {
        Ok(Self {
            database: DatabaseConfig::from_environment(environment)?,
            tokens: environment.generation_tokens()?,
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

pub(crate) struct AppState {
    pub(crate) config: Config,
    maintenance: AtomicBool,
    queue_lock: Mutex<()>,
    pub(crate) paths: StoragePaths,
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

    pub(crate) fn is_in_maintenance(&self) -> bool {
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

    pub(crate) fn save_generated_key(
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
                ON CONFLICT DO NOTHING
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

fn main() -> AppResult<()> {
    let environment = RuntimeEnvironment::load()?;
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    if run_developer_key_command(&arguments, &environment)? {
        return Ok(());
    }
    if arguments
        .iter()
        .any(|argument| argument == "--authorize-install")
    {
        Config::from_environment(&environment)?
            .database
            .require_active_installation()?;
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

fn run_developer_key_command(
    arguments: &[String],
    environment: &RuntimeEnvironment,
) -> AppResult<bool> {
    if !arguments
        .iter()
        .any(|argument| argument == DEV_GENERATE_KEY_FLAG)
    {
        return Ok(false);
    }
    let request_code = required_cli_option(arguments, "--request-code")?;
    let app_type = required_cli_option(arguments, "--app-type")?;
    let tokens = environment.generation_tokens()?;
    let activation_key = developer_key(&request_code, &app_type, &tokens)?;
    println!("{activation_key}");
    Ok(true)
}

fn required_cli_option(arguments: &[String], name: &str) -> AppResult<String> {
    let mut values = arguments
        .windows(2)
        .filter_map(|pair| (pair[0] == name).then(|| pair[1].clone()));
    let Some(value) = values.next() else {
        return Err(format!("Missing {name}.").into());
    };
    if values.next().is_some() {
        return Err(format!("Duplicate {name}.").into());
    }
    Ok(value)
}

fn developer_key(
    request_code: &str,
    app_type: &str,
    tokens: &[GenerationToken],
) -> AppResult<String> {
    let request_code = request_code.trim().to_ascii_uppercase();
    if !valid_request_code(&request_code) {
        return Err("Invalid request code format. Expected XXXX-XXXX-XXXX.".into());
    }
    generate_activation_key(&request_code, app_type, tokens)
        .ok_or_else(|| format!("Unknown app type: {app_type}.").into())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_keys_from_configured_tokens_only() {
        let tokens = vec![
            GenerationToken {
                id: "FIRST".to_owned(),
                name: "First Product".to_owned(),
                secret: "private-first-value".to_owned(),
            },
            GenerationToken {
                id: "SECOND".to_owned(),
                name: "Second Product".to_owned(),
                secret: "private-second-value".to_owned(),
            },
        ];
        let first = generate_activation_key("F81A-67A7-C6AA", "First Product", &tokens).unwrap();
        let first_again =
            generate_activation_key("F81A-67A7-C6AA", "First Product", &tokens).unwrap();
        let second = generate_activation_key("F81A-67A7-C6AA", "Second Product", &tokens).unwrap();

        assert_eq!(first, first_again);
        assert_ne!(first, second);
        assert_eq!(first.len(), 19);
        assert!(generate_activation_key("F81A-67A7-C6AA", "Unknown", &tokens).is_none());
    }

    #[test]
    fn validates_request_format() {
        assert!(valid_request_code("F81A-67A7-C6AA"));
        assert!(!valid_request_code("F81A67A7C6AA"));
    }

    #[test]
    fn developer_key_command_generates_without_database_or_logging() {
        let tokens = vec![GenerationToken {
            id: "JEWELRY".to_owned(),
            name: "Jewelry".to_owned(),
            secret: "JewelryManagement".to_owned(),
        }];

        let key = developer_key("2df9-3cf4-a810", "Jewelry", &tokens).unwrap();

        assert_eq!(key, "9BD1-121D-EBF6-017F");
    }

    #[test]
    fn cli_option_parser_rejects_missing_or_duplicate_options() {
        let arguments = vec![
            DEV_GENERATE_KEY_FLAG.to_owned(),
            "--request-code".to_owned(),
            "F81A-67A7-C6AA".to_owned(),
        ];

        assert_eq!(
            required_cli_option(&arguments, "--request-code").unwrap(),
            "F81A-67A7-C6AA"
        );
        assert!(required_cli_option(&arguments, "--app-type").is_err());

        let duplicate = vec![
            "--app-type".to_owned(),
            "Jewelry".to_owned(),
            "--app-type".to_owned(),
            "Lab".to_owned(),
        ];
        assert!(required_cli_option(&duplicate, "--app-type").is_err());
    }
}
