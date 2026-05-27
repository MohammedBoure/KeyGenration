import os
from contextlib import contextmanager

import psycopg2
from psycopg2.extras import RealDictCursor


REQUIRED_CONNECTION_SETTINGS = ("PGHOST", "PGPORT", "PGDATABASE", "PGUSER", "PGPASSWORD")


def get_db_connection():
    options = {
        "sslmode": os.environ.get("PGSSLMODE", "require"),
        "connect_timeout": int(os.environ.get("PGCONNECT_TIMEOUT", "10")),
    }
    database_url = os.environ.get("DATABASE_URL")
    if database_url:
        return psycopg2.connect(database_url, **options)

    settings = {
        name: os.environ.get(name, "").strip() for name in REQUIRED_CONNECTION_SETTINGS
    }
    missing = [name for name, value in settings.items() if not value]
    if missing:
        raise RuntimeError(
            f"{', '.join(missing)} must be defined in .env to connect to PostgreSQL."
        )

    return psycopg2.connect(
        host=settings["PGHOST"],
        port=settings["PGPORT"],
        dbname=settings["PGDATABASE"],
        user=settings["PGUSER"],
        password=settings["PGPASSWORD"],
        **options,
    )


@contextmanager
def db_cursor(dict_rows=False):
    connection = get_db_connection()
    cursor = (
        connection.cursor(cursor_factory=RealDictCursor)
        if dict_rows
        else connection.cursor()
    )
    try:
        yield cursor
        connection.commit()
    except Exception:
        connection.rollback()
        raise
    finally:
        cursor.close()
        connection.close()


def init_db():
    with db_cursor() as cursor:
        cursor.execute(
            """
            CREATE TABLE IF NOT EXISTS server_control (
                id INTEGER PRIMARY KEY,
                status TEXT NOT NULL CHECK (status IN ('0', '1'))
            )
            """
        )
        cursor.execute(
            """
            CREATE TABLE IF NOT EXISTS activation_logs (
                id BIGSERIAL PRIMARY KEY,
                sync_id TEXT UNIQUE,
                request_code TEXT,
                activation_key TEXT,
                generated_at TEXT,
                device_ip TEXT
            )
            """
        )
        cursor.execute("ALTER TABLE activation_logs ADD COLUMN IF NOT EXISTS sync_id TEXT")
        cursor.execute(
            """
            CREATE UNIQUE INDEX IF NOT EXISTS activation_logs_sync_id_idx
            ON activation_logs (sync_id)
            WHERE sync_id IS NOT NULL
            """
        )
        cursor.execute(
            """
            INSERT INTO server_control (id, status)
            VALUES (1, '1')
            ON CONFLICT (id) DO NOTHING
            """
        )
