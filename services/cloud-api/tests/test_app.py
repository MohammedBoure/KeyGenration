import os
import sys
import unittest
from contextlib import contextmanager
from pathlib import Path
from unittest.mock import patch


SERVICE_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SERVICE_ROOT))

from keygen_api import create_app


class CloudApiRoutingTests(unittest.TestCase):
    def setUp(self):
        self.previous_token = os.environ.get("KEYGEN_API_SECRET_TOKEN")
        self.previous_admin_token = os.environ.get("KEYGEN_ADMIN_SECRET_TOKEN")
        os.environ["KEYGEN_API_SECRET_TOKEN"] = "client-token"
        os.environ["KEYGEN_ADMIN_SECRET_TOKEN"] = "admin-token"
        application = create_app()
        application.testing = True
        self.client = application.test_client()

    def tearDown(self):
        if self.previous_token is None:
            os.environ.pop("KEYGEN_API_SECRET_TOKEN", None)
        else:
            os.environ["KEYGEN_API_SECRET_TOKEN"] = self.previous_token
        if self.previous_admin_token is None:
            os.environ.pop("KEYGEN_ADMIN_SECRET_TOKEN", None)
        else:
            os.environ["KEYGEN_ADMIN_SECRET_TOKEN"] = self.previous_admin_token

    def test_dashboard_template_is_available(self):
        response = self.client.get("/dashboard")
        self.assertEqual(response.status_code, 200)
        self.assertIn("KeyGen".encode(), response.data)

    def test_api_requires_bearer_token_before_database_access(self):
        response = self.client.get("/api/v1/server-status")
        self.assertEqual(response.status_code, 401)

    def test_authorized_status_request_preserves_api_contract(self):
        class Cursor:
            def execute(self, _query, _parameters=None):
                pass

            def fetchone(self):
                return ("1",)

        @contextmanager
        def cursor_context(_dict_rows=False):
            yield Cursor()

        with patch("keygen_api.api.db_cursor", cursor_context):
            response = self.client.get(
                "/api/v1/server-status",
                headers={"Authorization": "Bearer client-token"},
            )

        self.assertEqual(response.status_code, 200)
        self.assertEqual(response.get_json(), {"status": "1"})

    def test_client_token_cannot_enable_or_disable_generator(self):
        response = self.client.post(
            "/api/v1/set-status",
            json={"status": "0"},
            headers={"Authorization": "Bearer client-token"},
        )

        self.assertEqual(response.status_code, 403)

    def test_client_token_cannot_inject_activation_logs(self):
        get_response = self.client.get(
            "/api/v1/activation-logs",
            headers={"Authorization": "Bearer client-token"},
        )
        post_response = self.client.post(
            "/api/v1/activation-logs",
            json=[],
            headers={"Authorization": "Bearer client-token"},
        )

        self.assertEqual(get_response.status_code, 403)
        self.assertEqual(post_response.status_code, 405)

    def test_admin_token_can_disable_generator(self):
        statements = []

        class Cursor:
            def execute(self, query, parameters=None):
                statements.append((query, parameters))

        @contextmanager
        def cursor_context(_dict_rows=False):
            yield Cursor()

        with patch("keygen_api.api.db_cursor", cursor_context):
            response = self.client.post(
                "/api/v1/set-status",
                json={"status": "0"},
                headers={"Authorization": "Bearer admin-token"},
            )

        self.assertEqual(response.status_code, 200)
        self.assertEqual(statements[0][1], ("0",))

    def test_active_generator_creates_key_on_cloud(self):
        statements = []

        class Cursor:
            def execute(self, query, parameters=None):
                statements.append((query, parameters))

            def fetchone(self):
                return ("1",)

        @contextmanager
        def cursor_context(_dict_rows=False):
            yield Cursor()

        with patch("keygen_api.api.db_cursor", cursor_context):
            response = self.client.post(
                "/api/v1/generate-key",
                json={"request_code": "F81A-67A7-C6AA", "app_type": "Restaurant"},
                headers={"Authorization": "Bearer client-token"},
            )

        self.assertEqual(response.status_code, 200)
        self.assertEqual(response.get_json()["activation_key"], "EE8C-551F-0A90-73F5")
        self.assertEqual(len(statements), 2)

    def test_disabled_generator_does_not_create_key(self):
        statements = []

        class Cursor:
            def execute(self, query, parameters=None):
                statements.append((query, parameters))

            def fetchone(self):
                return ("0",)

        @contextmanager
        def cursor_context(_dict_rows=False):
            yield Cursor()

        with patch("keygen_api.api.db_cursor", cursor_context):
            response = self.client.post(
                "/api/v1/generate-key",
                json={"request_code": "F81A-67A7-C6AA", "app_type": "Restaurant"},
                headers={"Authorization": "Bearer client-token"},
            )

        self.assertEqual(response.status_code, 503)
        self.assertEqual(response.get_json()["status"], "maintenance_mode")
        self.assertEqual(len(statements), 1)


if __name__ == "__main__":
    unittest.main()
