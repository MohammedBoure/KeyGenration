use keygen_common::GenerationToken;
use sha2::{Digest, Sha256};

pub(crate) fn generate_activation_key(
    request_code: &str,
    app_type: &str,
    tokens: &[GenerationToken],
) -> Option<String> {
    let secret = &tokens.iter().find(|token| token.name == app_type)?.secret;
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

pub(crate) fn valid_request_code(request_code: &str) -> bool {
    request_code.len() == 14
        && request_code.as_bytes()[4] == b'-'
        && request_code.as_bytes()[9] == b'-'
}
