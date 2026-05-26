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
        os.environ["KEYGEN_API_SECRET_TOKEN"] = "test-token"
        application = create_app()
        application.testing = True
        self.client = application.test_client()

    def tearDown(self):
        if self.previous_token is None:
            os.environ.pop("KEYGEN_API_SECRET_TOKEN", None)
        else:
            os.environ["KEYGEN_API_SECRET_TOKEN"] = self.previous_token

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
                headers={"Authorization": "Bearer test-token"},
            )

        self.assertEqual(response.status_code, 200)
        self.assertEqual(response.get_json(), {"status": "1"})


if __name__ == "__main__":
    unittest.main()
