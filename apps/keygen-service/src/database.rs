use crate::AppResult;
use keygen_common::RuntimeEnvironment;
use native_tls::TlsConnector;
use postgres::config::SslMode;
use postgres::{Client, Config as PostgresConfig, NoTls};
use postgres_native_tls::MakeTlsConnector;
use std::time::Duration;

#[derive(Clone)]
pub(crate) struct DatabaseConfig {
    host: String,
    port: u16,
    database: String,
    user: String,
    password: String,
    ssl_mode: String,
    connect_timeout: Duration,
}

impl DatabaseConfig {
    pub(crate) fn from_environment(environment: &RuntimeEnvironment) -> AppResult<Self> {
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

    pub(crate) fn connect(&self) -> AppResult<Client> {
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

    pub(crate) fn read_status(&self) -> AppResult<String> {
        let mut client = self.connect()?;
        let row = client.query_opt("SELECT status FROM server_control WHERE id = 1", &[])?;
        row.map(|record| record.get::<_, String>(0))
            .ok_or_else(|| "The server_control status row is missing.".into())
    }

    pub(crate) fn require_active_installation(&self) -> AppResult<()> {
        match self.read_status()?.trim() {
            "1" => Ok(()),
            "0" => Err("Installation refusee: le generateur est desactive.".into()),
            _ => Err("Installation refusee: etat PostgreSQL invalide.".into()),
        }
    }
}

fn required_setting(environment: &RuntimeEnvironment, name: &str) -> AppResult<String> {
    environment
        .value(name)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("{name} is required for PostgreSQL synchronization.").into())
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
