import hmac
import os
from functools import wraps

from flask import jsonify, request


def _bearer_token():
    auth_header = request.headers.get("Authorization", "")
    prefix = "Bearer "
    if not auth_header.startswith(prefix):
        return None
    return auth_header[len(prefix) :]


def _require_auth(function, token_names):
    @wraps(function)
    def decorated(*args, **kwargs):
        expected_tokens = [
            os.environ[name] for name in token_names if os.environ.get(name)
        ]
        if not expected_tokens:
            return jsonify({"error": "Server authentication is not configured."}), 500

        supplied_token = _bearer_token()
        if supplied_token is None:
            return jsonify({"error": "Unauthorized. Missing or invalid token."}), 401

        if not any(
            hmac.compare_digest(supplied_token, expected_token)
            for expected_token in expected_tokens
        ):
            return jsonify({"error": "Forbidden. Invalid token."}), 403

        return function(*args, **kwargs)

    return decorated


def require_client_auth(function):
    return _require_auth(
        function,
        ("KEYGEN_API_SECRET_TOKEN", "KEYGEN_ADMIN_SECRET_TOKEN"),
    )


def require_admin_auth(function):
    return _require_auth(function, ("KEYGEN_ADMIN_SECRET_TOKEN",))
