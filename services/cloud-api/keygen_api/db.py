import os
from contextlib import contextmanager

import psycopg2
from psycopg2.extras import RealDictCursor


DEFAULT_DB_HOST = "sw4.duckdns.org"
DEFAULT_DB_PORT = "9005"
DEFAULT_DB_NAME = "keygen_restaurant"
DEFAULT_DB_USER = "keygen_app"


def get_db_connection():
    options = {
        "sslmode": os.environ.get("PGSSLMODE", "require"),
        "connect_timeout": int(os.environ.get("PGCONNECT_TIMEOUT", "10")),
    }
    database_url = os.environ.get("DATABASE_URL")
    if database_url:
        return psycopg2.connect(database_url, **options)

    password = os.environ.get("PGPASSWORD")
    if not password:
        raise RuntimeError("PGPASSWORD is required to connect to PostgreSQL.")

    return psycopg2.connect(
        host=os.environ.get("PGHOST", DEFAULT_DB_HOST),
        port=os.environ.get("PGPORT", DEFAULT_DB_PORT),
        dbname=os.environ.get("PGDATABASE", DEFAULT_DB_NAME),
        user=os.environ.get("PGUSER", DEFAULT_DB_USER),
        password=password,
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
                request_code TEXT,
                activation_key TEXT,
                generated_at TEXT,
                device_ip TEXT
            )
            """
        )
        cursor.execute(
            """
            INSERT INTO server_control (id, status)
            VALUES (1, '1')
            ON CONFLICT (id) DO NOTHING
            """
        )
