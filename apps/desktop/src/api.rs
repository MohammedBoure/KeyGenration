use crate::AppResult;
use keygen_common::{GenerationToken, RuntimeEnvironment};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::net::SocketAddr;
use std::time::Duration;

const DEFAULT_LOCAL_LISTEN_ADDRESS: &str = "127.0.0.1:45632";
const REQUIRED_BACKEND_MODE: &str = "local-queue-v1";

#[derive(Deserialize)]
struct GenerateResponse {
    activation_key: String,
}

#[derive(Deserialize)]
struct HealthResponse {
    backend_mode: Option<String>,
    token_names: Option<Vec<String>>,
}

#[derive(Serialize)]
struct GenerateRequest<'a> {
    request_code: &'a str,
    app_type: &'a str,
}

fn configured_local_generate_url() -> AppResult<String> {
    configured_local_endpoint_url("generate_key")
}

fn configured_local_health_url() -> AppResult<String> {
    configured_local_endpoint_url("health")
}

fn configured_local_endpoint_url(endpoint: &str) -> AppResult<String> {
    let listen_address = configured_value("KEYGEN_LISTEN_ADDRESS")
        .unwrap_or_else(|| DEFAULT_LOCAL_LISTEN_ADDRESS.to_owned());
    local_endpoint_url(&listen_address, endpoint)
}

pub(crate) fn local_endpoint_url(listen_address: &str, endpoint: &str) -> AppResult<String> {
    let address: SocketAddr = listen_address
        .trim()
        .parse()
        .map_err(|_| "KEYGEN_LISTEN_ADDRESS doit etre une adresse locale valide.".to_owned())?;
    Ok(format!("http://{address}/{endpoint}"))
}

fn configured_value(name: &str) -> Option<String> {
    RuntimeEnvironment::load()
        .ok()
        .and_then(|environment| environment.value(name))
}

fn runtime_environment() -> AppResult<RuntimeEnvironment> {
    RuntimeEnvironment::load()
        .map_err(|error| format!("Lecture du fichier .env impossible: {error}"))
}

fn configured_generation_tokens() -> AppResult<Vec<GenerationToken>> {
    runtime_environment()?
        .generation_tokens()
        .map_err(|error| format!("Configuration des tokens invalide: {error}"))
}

pub(crate) fn configured_token_names() -> AppResult<Vec<String>> {
    Ok(configured_generation_tokens()?
        .into_iter()
        .map(|token| token.name)
        .collect())
}

pub(crate) fn validate_request_code(value: &str) -> AppResult<String> {
    let request_code = value.trim().to_ascii_uppercase();
    if request_code.len() == 14
        && request_code.as_bytes()[4] == b'-'
        && request_code.as_bytes()[9] == b'-'
    {
        Ok(request_code)
    } else {
        Err("Format du code: XXXX-XXXX-XXXX.".to_owned())
    }
}

pub(crate) fn local_service_ready() -> AppResult<()> {
    let expected_token_names = configured_token_names()?;
    let response = ureq::get(&configured_local_health_url()?)
        .timeout(Duration::from_secs(2))
        .call()
        .map_err(|error| format!("Service local indisponible: {error}"))?;
    let health: HealthResponse = response
        .into_json()
        .map_err(|error| format!("Reponse du service local invalide: {error}"))?;
    if health.backend_mode.as_deref() != Some(REQUIRED_BACKEND_MODE) {
        return Err("Service local obsolete; mise a jour requise.".to_owned());
    }
    match health.token_names {
        Some(names) if names == expected_token_names => Ok(()),
        _ => Err("Configuration des tokens modifiee; mise a jour du service requise.".to_owned()),
    }
}

pub(crate) fn generate_key(request_code: &str, app_type: &str) -> AppResult<String> {
    let payload = GenerateRequest {
        request_code,
        app_type,
    };
    let response = ureq::post(&configured_local_generate_url()?)
        .set("Content-Type", "application/json")
        .timeout(Duration::from_secs(10))
        .send_json(json!(payload))
        .map_err(|error| format!("Service local indisponible: {error}"))?;
    let response: GenerateResponse = response
        .into_json()
        .map_err(|error| format!("Reponse locale invalide: {error}"))?;
    Ok(response.activation_key)
}
