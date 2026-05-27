import sys
import unittest
from contextlib import contextmanager
from pathlib import Path
from unittest.mock import patch

from fastapi.testclient import TestClient


SERVICE_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SERVICE_ROOT))

from keygen_api.admin_web import create_admin_app


class AdminWebTests(unittest.TestCase):
    def setUp(self):
        self.client = TestClient(create_admin_app(initialize_database=False))

    def test_dashboard_page_is_available(self):
        response = self.client.get("/")

        self.assertEqual(response.status_code, 200)
        self.assertIn("لوحة متابعة".encode("utf-8"), response.content)
        delete_function = response.text.split("async function deleteRecord", 1)[1].split(
            "document.getElementById", 1
        )[0]
        self.assertEqual(delete_function.count("window.confirm("), 2)

    def test_dashboard_rejects_non_local_requests_by_default(self):
        remote_client = TestClient(
            create_admin_app(initialize_database=False),
            client=("203.0.113.20", 50000),
        )

        with patch.dict("os.environ", {"ADMIN_WEB_ALLOW_REMOTE": "0"}):
            response = remote_client.get("/health")

        self.assertEqual(response.status_code, 403)

    def test_status_reads_control_value(self):
        class Cursor:
            def execute(self, _query, _parameters=None):
                pass

            def fetchone(self):
                return ("1",)

        @contextmanager
        def cursor_context(_dict_rows=False):
            yield Cursor()

        with patch("keygen_api.admin_web.db.db_cursor", cursor_context):
            response = self.client.get("/api/status")

        self.assertEqual(response.status_code, 200)
        self.assertEqual(response.json(), {"status": "1"})

    def test_status_update_accepts_only_valid_control_value(self):
        statements = []

        class Cursor:
            def execute(self, query, parameters=None):
                statements.append((query, parameters))

        @contextmanager
        def cursor_context(_dict_rows=False):
            yield Cursor()

        with patch("keygen_api.admin_web.db.db_cursor", cursor_context):
            accepted = self.client.put("/api/status", json={"status": "0"})
            rejected = self.client.put("/api/status", json={"status": "2"})

        self.assertEqual(accepted.status_code, 200)
        self.assertEqual(accepted.json(), {"status": "0"})
        self.assertEqual(statements[0][1], ("0",))
        self.assertEqual(rejected.status_code, 422)
        self.assertEqual(len(statements), 1)

    def test_activation_records_are_returned_from_database(self):
        class Cursor:
            def execute(self, _query, parameters=None):
                self.parameters = parameters

            def fetchall(self):
                return [
                    {
                        "id": 4,
                        "request_code": "F81A-67A7-C6AA",
                        "activation_key": "EE8C-551F-0A90-73F5",
                        "generated_at": "2026-05-26T16:00:00+00:00",
                        "device_ip": "127.0.0.1",
                    }
                ]

        @contextmanager
        def cursor_context(dict_rows=False):
            self.assertTrue(dict_rows)
            yield Cursor()

        with patch("keygen_api.admin_web.db.db_cursor", cursor_context):
            response = self.client.get("/api/activation-logs?limit=25")

        self.assertEqual(response.status_code, 200)
        self.assertEqual(response.json()["count"], 1)
        self.assertEqual(response.json()["records"][0]["id"], 4)

    def test_activation_record_can_be_deleted_by_id(self):
        statements = []

        class Cursor:
            def execute(self, query, parameters=None):
                statements.append((query, parameters))

            def fetchone(self):
                return (4,)

        @contextmanager
        def cursor_context(_dict_rows=False):
            yield Cursor()

        with patch("keygen_api.admin_web.db.db_cursor", cursor_context):
            response = self.client.delete("/api/activation-logs/4")

        self.assertEqual(response.status_code, 200)
        self.assertEqual(response.json(), {"deleted": True, "id": 4})
        self.assertIn("DELETE FROM activation_logs", statements[0][0])
        self.assertEqual(statements[0][1], (4,))

    def test_deleting_missing_activation_record_returns_not_found(self):
        class Cursor:
            def execute(self, _query, _parameters=None):
                pass

            def fetchone(self):
                return None

        @contextmanager
        def cursor_context(_dict_rows=False):
            yield Cursor()

        with patch("keygen_api.admin_web.db.db_cursor", cursor_context):
            response = self.client.delete("/api/activation-logs/77")

        self.assertEqual(response.status_code, 404)


if __name__ == "__main__":
    unittest.main()
