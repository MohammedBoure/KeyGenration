from datetime import datetime, timezone

from flask import Blueprint, jsonify, request

from .auth import require_admin_auth, require_client_auth
from .db import db_cursor
from .generator import generate_activation_key, normalize_request_code


api = Blueprint("api", __name__, url_prefix="/api/v1")


@api.get("/server-status")
@require_client_auth
def get_server_status():
    try:
        with db_cursor() as cursor:
            cursor.execute("SELECT status FROM server_control WHERE id = 1")
            result = cursor.fetchone()

        if result:
            return jsonify({"status": result[0]}), 200
        return jsonify({"status": "0", "error": "Status record not found"}), 500
    except Exception as error:
        return jsonify({"error": str(error)}), 500


@api.post("/set-status")
@require_admin_auth
def set_server_status():
    data = request.get_json(silent=True) or {}
    new_status = str(data.get("status", "")).strip()
    if new_status not in {"0", "1"}:
        return jsonify({"error": "Invalid status. Use '1' (Active) or '0' (Maintenance)."}), 400

    try:
        with db_cursor() as cursor:
            cursor.execute(
                "UPDATE server_control SET status = %s WHERE id = 1", (new_status,)
            )
        return jsonify({"message": f"Server status updated to {new_status}"}), 200
    except Exception as error:
        return jsonify({"error": str(error)}), 500


@api.post("/generate-key")
@require_client_auth
def generate_key():
    data = request.get_json(silent=True) or {}
    try:
        request_code = normalize_request_code(data.get("request_code"))
        app_type = str(data.get("app_type", "")).strip()
        activation_key = generate_activation_key(request_code, app_type)
    except ValueError as error:
        return jsonify({"error": str(error)}), 400

    try:
        with db_cursor() as cursor:
            cursor.execute("SELECT status FROM server_control WHERE id = 1 FOR SHARE")
            status = cursor.fetchone()
            if not status or status[0] != "1":
                return (
                    jsonify(
                        {
                            "error": "Key generation is disabled by the administrator.",
                            "status": "maintenance_mode",
                        }
                    ),
                    503,
                )
            cursor.execute(
                """
                INSERT INTO activation_logs
                    (request_code, activation_key, generated_at, device_ip)
                VALUES (%s, %s, %s, %s)
                """,
                (
                    request_code,
                    activation_key,
                    datetime.now(timezone.utc).isoformat(),
                    request.remote_addr,
                ),
            )
        return (
            jsonify(
                {
                    "request_code": request_code,
                    "activation_key": activation_key,
                    "app_type": app_type,
                    "status": "generated_and_recorded",
                }
            ),
            200,
        )
    except Exception as error:
        return jsonify({"error": str(error)}), 500


@api.get("/activation-logs")
@require_admin_auth
def get_activation_logs():
    try:
        with db_cursor(dict_rows=True) as cursor:
            cursor.execute("SELECT * FROM activation_logs ORDER BY id DESC")
            rows = cursor.fetchall()
        return jsonify([dict(row) for row in rows]), 200
    except Exception as error:
        return jsonify({"error": str(error)}), 500
