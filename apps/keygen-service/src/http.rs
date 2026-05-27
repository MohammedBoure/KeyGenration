use crate::AppState;
use crate::generation::{generate_activation_key, valid_request_code};
use crate::storage::load_pending_uploads;
use serde::Deserialize;
use serde_json::{Value, json};
use tiny_http::{Header, Method, Request, Response, StatusCode};

const BACKEND_MODE: &str = "local-queue-v1";

#[derive(Debug, Deserialize)]
struct GenerateRequest {
    request_code: Option<String>,
    app_type: Option<String>,
}

pub(crate) fn handle_request(mut request: Request, state: &AppState) {
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
                "token_names": state.config.tokens.iter()
                    .map(|token| token.name.as_str())
                    .collect::<Vec<_>>(),
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

fn generate_key_response(request: &mut Request, state: &AppState) -> (u16, Value) {
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
    let app_type = payload
        .app_type
        .as_deref()
        .unwrap_or_else(|| state.config.tokens[0].name.as_str());
    let Some(activation_key) =
        generate_activation_key(&request_code, app_type, &state.config.tokens)
    else {
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
