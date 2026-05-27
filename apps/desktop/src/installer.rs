use crate::{AppResult, SERVICE_NAME};
use keygen_common::{RuntimeEnvironment, TOKEN_IDS_VARIABLE, env_assignment};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

const SERVICE_ENV_OPTIONS: [&str; 12] = [
    "PGHOST",
    "PGPORT",
    "PGDATABASE",
    "PGUSER",
    "PGPASSWORD",
    "PGSSLMODE",
    "PGCONNECT_TIMEOUT",
    "KEYGEN_LISTEN_ADDRESS",
    "KEYGEN_DATA_DIR",
    "KEYGEN_PRIMARY_LOG",
    "KEYGEN_STATUS_INTERVAL_SECONDS",
    "KEYGEN_UPLOAD_INTERVAL_SECONDS",
];

fn runtime_environment() -> AppResult<RuntimeEnvironment> {
    RuntimeEnvironment::load()
        .map_err(|error| format!("Lecture du fichier .env impossible: {error}"))
}

fn bundle_root() -> AppResult<PathBuf> {
    if let Some(path) = env::var_os("ACTIVATEUR_BUNDLE_DIR") {
        return Ok(PathBuf::from(path));
    }
    let executable = env::current_exe()
        .map_err(|error| format!("Impossible de localiser l'application: {error}"))?;
    executable
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| "Dossier de l'application introuvable.".to_owned())
}

fn install_root() -> AppResult<PathBuf> {
    env::var_os("ProgramFiles")
        .map(PathBuf::from)
        .map(|path| path.join("KeyGenRMS"))
        .ok_or_else(|| "Variable ProgramFiles introuvable.".to_owned())
}

fn run_command(program: &Path, arguments: &[&str]) -> AppResult<()> {
    let output = Command::new(program)
        .args(arguments)
        .output()
        .map_err(|error| format!("Echec de lancement de {}: {error}", program.display()))?;
    if output.status.success() {
        return Ok(());
    }
    let details = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    Err(format!(
        "Commande echouee ({}): {}",
        program.display(),
        details
    ))
}

fn run_optional(program: &Path, arguments: &[&str]) {
    let _ = Command::new(program).args(arguments).output();
}

pub(crate) fn bundled_service_path() -> AppResult<PathBuf> {
    let service = bundle_root()?
        .join("KeyGenService")
        .join("KeyGenService.exe");
    if !service.is_file() {
        return Err(format!("Backend Rust manquant: {}", service.display()));
    }
    Ok(service)
}

fn bundled_nssm_path() -> AppResult<PathBuf> {
    let nssm = bundle_root()?.join("nssm").join("nssm.exe");
    if !nssm.is_file() {
        return Err(format!("NSSM manquant: {}", nssm.display()));
    }
    Ok(nssm)
}

pub(crate) fn verify_install_authorization(service: &Path) -> AppResult<()> {
    let environment = runtime_environment()?;
    let mut command = Command::new(service);
    command.arg("--authorize-install");
    if environment.exists() {
        command.env("KEYGEN_ENV_FILE", environment.path());
    }
    let output = command
        .output()
        .map_err(|error| format!("Verification PostgreSQL impossible: {error}"))?;
    if output.status.success() {
        return Ok(());
    }
    let details = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    if details.is_empty() {
        Err("Installation interdite ou connexion PostgreSQL indisponible.".to_owned())
    } else {
        Err(details)
    }
}

fn service_environment_contents() -> AppResult<String> {
    let source_environment = runtime_environment()?;
    let tokens = source_environment
        .generation_tokens()
        .map_err(|error| format!("Configuration des tokens invalide: {error}"))?;
    let mut contents =
        "# Generated local service settings for PostgreSQL synchronization.\n".to_owned();
    for name in SERVICE_ENV_OPTIONS {
        if let Some(value) = source_environment.value(name) {
            contents.push_str(&env_assignment(name, &value));
        }
    }
    let token_ids = tokens
        .iter()
        .map(|token| token.id.as_str())
        .collect::<Vec<_>>()
        .join(",");
    contents.push_str(&env_assignment(TOKEN_IDS_VARIABLE, &token_ids));
    for token in tokens {
        contents.push_str(&env_assignment(&token.name_variable(), &token.name));
        contents.push_str(&env_assignment(&token.secret_variable(), &token.secret));
    }
    Ok(contents)
}

pub(crate) fn install_service() -> AppResult<String> {
    let source_service = bundled_service_path()?;
    let source_nssm = bundled_nssm_path()?;
    verify_install_authorization(&source_service)?;

    let destination = install_root()?;
    fs::create_dir_all(&destination)
        .map_err(|error| format!("Creation du dossier impossible: {error}"))?;
    let installed_service = destination.join("KeyGenService.exe");
    let installed_nssm = destination.join("nssm.exe");

    // Windows keeps a running executable locked; stop an existing instance before upgrading it.
    run_optional(&source_nssm, &["stop", SERVICE_NAME, "confirm"]);
    run_optional(&source_nssm, &["remove", SERVICE_NAME, "confirm"]);
    fs::copy(&source_service, &installed_service)
        .map_err(|error| format!("Copie de KeyGenService impossible: {error}"))?;
    fs::copy(&source_nssm, &installed_nssm)
        .map_err(|error| format!("Copie de NSSM impossible: {error}"))?;
    fs::write(destination.join(".env"), service_environment_contents()?)
        .map_err(|error| format!("Ecriture du fichier .env du service impossible: {error}"))?;

    let service_path = installed_service.to_string_lossy().to_string();
    let app_directory = destination.to_string_lossy().to_string();

    run_command(&installed_nssm, &["install", SERVICE_NAME, &service_path])?;
    run_command(
        &installed_nssm,
        &["set", SERVICE_NAME, "AppDirectory", &app_directory],
    )?;
    run_command(
        &installed_nssm,
        &["set", SERVICE_NAME, "Start", "SERVICE_AUTO_START"],
    )?;
    run_command(&installed_nssm, &["set", SERVICE_NAME, "AppNoConsole", "1"])?;
    run_command(&installed_nssm, &["start", SERVICE_NAME])?;

    Ok("Service Rust installe et demarre.".to_owned())
}

pub(crate) fn uninstall_service() -> AppResult<String> {
    let source_nssm = bundled_nssm_path()?;
    run_optional(&source_nssm, &["stop", SERVICE_NAME, "confirm"]);
    run_optional(&source_nssm, &["remove", SERVICE_NAME, "confirm"]);
    std::thread::sleep(Duration::from_millis(500));

    let destination = install_root()?;
    if destination.exists() {
        fs::remove_dir_all(&destination)
            .map_err(|error| format!("Suppression du backend local impossible: {error}"))?;
    }
    Ok("Service backend local supprime. Les donnees locales sont conservees.".to_owned())
}
