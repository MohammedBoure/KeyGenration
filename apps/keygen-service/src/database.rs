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
                config
                    .connect(NoTls)
                    .map_err(|error| self.connection_error(&error).into())
            }
            "require" => {
                config.ssl_mode(SslMode::Require);
                let mut tls = TlsConnector::builder();
                // PostgreSQL sslmode=require encrypts transport without validating its certificate.
                tls.danger_accept_invalid_certs(true);
                let tls = tls.build()?;
                config
                    .connect(MakeTlsConnector::new(tls))
                    .map_err(|error| self.connection_error(&error).into())
            }
            "verify-full" => {
                config.ssl_mode(SslMode::Require);
                let tls = TlsConnector::new()?;
                config
                    .connect(MakeTlsConnector::new(tls))
                    .map_err(|error| self.connection_error(&error).into())
            }
            _ => Err("PGSSLMODE must be disable, require, or verify-full.".into()),
        }
    }

    fn connection_error(&self, error: &dyn std::error::Error) -> String {
        let details = error_chain(error);
        format!(
            "Connexion PostgreSQL impossible vers {}:{} apres {}s. Verifiez Internet, PGHOST/PGPORT, le pare-feu et que le serveur PostgreSQL accepte les connexions. Detail: {details}",
            self.host,
            self.port,
            self.connect_timeout.as_secs()
        )
    }

    pub(crate) fn read_status(&self) -> AppResult<String> {
        let mut client = self.connect()?;
        let row = client
            .query_opt("SELECT status FROM server_control WHERE id = 1", &[])
            .map_err(|error| format!("Lecture du statut PostgreSQL impossible: {error}"))?;
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

fn error_chain(error: &dyn std::error::Error) -> String {
    let mut details = error.to_string();
    let mut source = error.source();
    while let Some(error) = source {
        details.push_str(": ");
        details.push_str(&error.to_string());
        source = error.source();
    }
    details
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connection_errors_include_target_and_timeout() {
        let config = DatabaseConfig {
            host: "database.example.test".to_owned(),
            port: 5432,
            database: "keygen".to_owned(),
            user: "client".to_owned(),
            password: "secret".to_owned(),
            ssl_mode: "require".to_owned(),
            connect_timeout: Duration::from_secs(7),
        };

        let error = std::io::Error::new(std::io::ErrorKind::TimedOut, "connection timed out");
        let message = config.connection_error(&error);

        assert!(message.contains("database.example.test:5432"));
        assert!(message.contains("apres 7s"));
        assert!(message.contains("connection timed out"));
        assert!(!message.contains("secret"));
    }
}
