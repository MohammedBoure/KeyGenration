import hmac
import os
from functools import wraps

from flask import jsonify, request


def require_auth(function):
    @wraps(function)
    def decorated(*args, **kwargs):
        expected_token = os.environ.get("KEYGEN_API_SECRET_TOKEN")
        if not expected_token:
            return jsonify({"error": "Server authentication is not configured."}), 500

        auth_header = request.headers.get("Authorization", "")
        prefix = "Bearer "
        if not auth_header.startswith(prefix):
            return jsonify({"error": "Unauthorized. Missing or invalid token."}), 401

        supplied_token = auth_header[len(prefix) :]
        if not hmac.compare_digest(supplied_token, expected_token):
            return jsonify({"error": "Forbidden. Invalid token."}), 403

        return function(*args, **kwargs)

    return decorated
