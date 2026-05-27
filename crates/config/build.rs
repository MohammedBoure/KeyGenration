use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const BUILD_ENV_VARIABLE: &str = "KEYGEN_BUILD_ENV_FILE";
const CLIENT_SETTINGS: [&str; 10] = [
    "PGHOST",
    "PGPORT",
    "PGDATABASE",
    "PGSSLMODE",
    "PGCONNECT_TIMEOUT",
    "KEYGEN_LISTEN_ADDRESS",
    "KEYGEN_DATA_DIR",
    "KEYGEN_PRIMARY_LOG",
    "KEYGEN_STATUS_INTERVAL_SECONDS",
    "KEYGEN_UPLOAD_INTERVAL_SECONDS",
];

fn main() {
    println!("cargo:rerun-if-env-changed={BUILD_ENV_VARIABLE}");

    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let configured_path = env::var_os(BUILD_ENV_VARIABLE)
        .map(PathBuf::from)
        .unwrap_or_else(|| manifest_dir.join("../../packaging/windows/.env"));
    println!("cargo:rerun-if-changed={}", configured_path.display());

    let filtered = read_filtered_client_configuration(&configured_path);
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let generated = format!("pub const EMBEDDED_CLIENT_ENV: &str = {filtered:?};\n");
    fs::write(out_dir.join("embedded_client_env.rs"), generated)
        .expect("unable to write generated embedded client configuration");
}

fn read_filtered_client_configuration(path: &Path) -> String {
    let Ok(contents) = fs::read_to_string(path) else {
        return String::new();
    };

    let mut filtered = String::new();
    for source_line in contents.lines() {
        let line = source_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let line = line.strip_prefix("export ").unwrap_or(line).trim();
        let Some((name, _value)) = line.split_once('=') else {
            continue;
        };
        if CLIENT_SETTINGS.contains(&name.trim()) {
            filtered.push_str(line);
            filtered.push('\n');
        }
    }
    filtered
}
