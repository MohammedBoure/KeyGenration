from flask import Blueprint, jsonify, request

from .auth import require_auth
from .db import db_cursor


api = Blueprint("api", __name__, url_prefix="/api/v1")


@api.get("/server-status")
@require_auth
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
@require_auth
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


@api.route("/activation-logs", methods=["POST", "GET"])
@require_auth
def activation_logs():
    if request.method == "POST":
        records = request.get_json(silent=True)
        if not isinstance(records, list):
            return jsonify({"error": "Expected a list of JSON objects"}), 400
        try:
            with db_cursor() as cursor:
                for item in records:
                    cursor.execute(
                        """
                        INSERT INTO activation_logs
                            (request_code, activation_key, generated_at, device_ip)
                        VALUES (%s, %s, %s, %s)
                        """,
                        (
                            item.get("request_code"),
                            item.get("activation_key"),
                            item.get("generated_at"),
                            item.get("device_ip"),
                        ),
                    )
            return jsonify({"message": f"Successfully inserted {len(records)} records."}), 201
        except Exception as error:
            return jsonify({"error": str(error)}), 500

    try:
        with db_cursor(dict_rows=True) as cursor:
            cursor.execute("SELECT * FROM activation_logs ORDER BY id DESC")
            rows = cursor.fetchall()
        return jsonify([dict(row) for row in rows]), 200
    except Exception as error:
        return jsonify({"error": str(error)}), 500
