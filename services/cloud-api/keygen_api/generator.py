import hashlib


APP_SECRETS = {
    "Restaurant": "RestaurantManagement",
    "Lab": "LabInventoryManagement",
    "Jewelry": "JewelryManagement",
}


def normalize_request_code(value):
    request_code = str(value or "").strip().upper()
    if (
        len(request_code) != 14
        or request_code[4] != "-"
        or request_code[9] != "-"
    ):
        raise ValueError("Invalid request_code format. Expected 'XXXX-XXXX-XXXX'.")
    return request_code


def generate_activation_key(request_code, app_type):
    secret = APP_SECRETS.get(app_type)
    if secret is None:
        raise ValueError("Invalid app_type. Use Restaurant, Lab, or Jewelry.")
    digest = hashlib.sha256(f"{request_code}::{secret}".encode()).hexdigest().upper()
    value = digest[:16]
    return "-".join(value[index : index + 4] for index in range(0, 16, 4))
