import os
import sys
import unittest
from pathlib import Path
from unittest.mock import patch


SERVICE_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SERVICE_ROOT))

from keygen_api import db


class DatabaseConfigurationTests(unittest.TestCase):
    def test_connection_requires_all_private_environment_settings(self):
        with patch.dict(os.environ, {}, clear=True):
            with self.assertRaisesRegex(RuntimeError, "PGHOST, PGPORT, PGDATABASE, PGUSER, PGPASSWORD"):
                db.get_db_connection()

    def test_connection_uses_explicit_environment_settings(self):
        configuration = {
            "PGHOST": "private.database.test",
            "PGPORT": "6543",
            "PGDATABASE": "private_database",
            "PGUSER": "private_user",
            "PGPASSWORD": "private_password",
            "PGSSLMODE": "require",
            "PGCONNECT_TIMEOUT": "4",
        }

        with (
            patch.dict(os.environ, configuration, clear=True),
            patch("keygen_api.db.psycopg2.connect") as connect,
        ):
            db.get_db_connection()

        connect.assert_called_once_with(
            host="private.database.test",
            port="6543",
            dbname="private_database",
            user="private_user",
            password="private_password",
            sslmode="require",
            connect_timeout=4,
        )


if __name__ == "__main__":
    unittest.main()
