import argparse

from keygen_api import create_app
from keygen_api.config import debug_enabled, load_environment
from keygen_api.db import init_db
from keygen_api.migrations import migrate_from_sqlite


def parse_args():
    parser = argparse.ArgumentParser(description="Activateur RMS cloud API")
    parser.add_argument(
        "--migrate-sqlite",
        metavar="PATH",
        help="Import activation records from an existing SQLite database.",
    )
    parser.add_argument(
        "--init-db-only",
        action="store_true",
        help="Initialize PostgreSQL tables without starting the HTTP API.",
    )
    return parser.parse_args()


def main():
    load_environment()
    args = parse_args()
    init_db()

    if args.migrate_sqlite:
        count = migrate_from_sqlite(args.migrate_sqlite)
        print(f"Imported or retained {count} SQLite activation log records.")
        return
    if args.init_db_only:
        print("PostgreSQL tables are initialized.")
        return

    create_app().run(host="0.0.0.0", port=7002, debug=debug_enabled())


if __name__ == "__main__":
    main()
