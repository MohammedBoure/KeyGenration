import sqlite3

from .db import db_cursor


def migrate_from_sqlite(sqlite_path):
    source = sqlite3.connect(sqlite_path)
    source.row_factory = sqlite3.Row
    try:
        status_rows = source.execute("SELECT id, status FROM server_control").fetchall()
        log_rows = source.execute(
            """
            SELECT id, request_code, activation_key, generated_at, device_ip
            FROM activation_logs
            ORDER BY id
            """
        ).fetchall()
    finally:
        source.close()

    with db_cursor() as cursor:
        for row in status_rows:
            cursor.execute(
                """
                INSERT INTO server_control (id, status)
                VALUES (%s, %s)
                ON CONFLICT (id) DO UPDATE SET status = EXCLUDED.status
                """,
                (row["id"], row["status"]),
            )

        for row in log_rows:
            cursor.execute(
                """
                INSERT INTO activation_logs
                    (id, request_code, activation_key, generated_at, device_ip)
                VALUES (%s, %s, %s, %s, %s)
                ON CONFLICT (id) DO NOTHING
                """,
                (
                    row["id"],
                    row["request_code"],
                    row["activation_key"],
                    row["generated_at"],
                    row["device_ip"],
                ),
            )

        cursor.execute(
            """
            SELECT setval(
                pg_get_serial_sequence('activation_logs', 'id'),
                COALESCE((SELECT MAX(id) FROM activation_logs), 1),
                EXISTS (SELECT 1 FROM activation_logs)
            )
            """
        )

    return len(log_rows)
