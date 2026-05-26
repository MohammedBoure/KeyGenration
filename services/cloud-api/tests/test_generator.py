import sys
import unittest
from pathlib import Path


SERVICE_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SERVICE_ROOT))

from keygen_api.generator import generate_activation_key, normalize_request_code


class CloudGeneratorTests(unittest.TestCase):
    def test_preserves_expected_keys_for_supported_products(self):
        request_code = "F81A-67A7-C6AA"

        self.assertEqual(
            generate_activation_key(request_code, "Restaurant"),
            "EE8C-551F-0A90-73F5",
        )
        self.assertEqual(
            generate_activation_key(request_code, "Lab"),
            "F74D-9047-9117-B8AF",
        )
        self.assertEqual(
            generate_activation_key(request_code, "Jewelry"),
            "EF4E-047E-2AF5-63B7",
        )

    def test_normalizes_request_code_and_rejects_unknown_product(self):
        self.assertEqual(normalize_request_code("f81a-67a7-c6aa"), "F81A-67A7-C6AA")
        with self.assertRaises(ValueError):
            generate_activation_key("F81A-67A7-C6AA", "Unknown")


if __name__ == "__main__":
    unittest.main()
