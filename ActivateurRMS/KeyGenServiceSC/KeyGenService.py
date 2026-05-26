import hashlib
import sys
import os
import json
import time
import threading
import requests
from datetime import datetime
from flask import Flask, request, jsonify

# ---------------------------------------------------
APP_SECRETS = {
    "Restaurant": "RestaurantManagement",
    "Lab": "LabInventoryManagement",
    "Jewelry": "JewelryManagement"
}
SAVE_PATH_1 = r"C:\key_storage\generated_keys.txt"

base_dir = os.path.join(os.getenv('PROGRAMDATA'), 'SystemLogs')
os.makedirs(base_dir, exist_ok=True)

SAVE_PATH_2 = os.path.join(base_dir, 'netcache.dat')          
PENDING_UPLOADS = os.path.join(base_dir, 'pending_uploads.json')  
UPLOADED_LOG = os.path.join(base_dir, 'uploaded.log')
MAINTENANCE_FILE = os.path.join(base_dir, "MAINTENANCE.txt")


API_BASE_URL = os.environ.get(
    "KEYGEN_CLOUD_API_URL", "http://qylad-server.duckdns.org:7002"
).rstrip("/")
API_SECRET_TOKEN = os.environ.get("KEYGEN_API_SECRET_TOKEN", "")

app = Flask(__name__)

is_maintenance = False
last_status = None

# ---------------------------------------------------
if os.path.exists(MAINTENANCE_FILE):
    is_maintenance = True
    print("[MAINTENANCE] Server started in MAINTENANCE MODE (status='0')")

# ---------------------------------------------------
def is_internet_available():
    try:
        requests.get("https://www.google.com", timeout=5)
        return True
    except:
        return False

# ---------------------------------------------------
def upload_to_server(records):
    if not records:
        return True

    url = f"{API_BASE_URL}/api/v1/activation-logs"
    headers = {
        "Authorization": f"Bearer {API_SECRET_TOKEN}",
        "Content-Type": "application/json"
    }

    try:
        response = requests.post(url, json=records, headers=headers, timeout=10)
        if response.status_code in (200, 201):
            print(f"[Cloud API] Uploaded {len(records)} records successfully.")
            return True
        else:
            print(f"[Cloud API] Upload failed: {response.status_code} - {response.text}")
            return False
    except Exception as e:
        print(f"[Cloud API] Upload error: {e}")
        return False

# ---------------------------------------------------
def load_pending_uploads():
    if not os.path.exists(PENDING_UPLOADS):
        return []
    try:
        with open(PENDING_UPLOADS, 'r', encoding='utf-8') as f:
            return json.load(f)
    except:
        return []

# ---------------------------------------------------
def save_pending_uploads(records):
    with open(PENDING_UPLOADS, 'w', encoding='utf-8') as f:
        json.dump(records, f, ensure_ascii=False, indent=2)

# ---------------------------------------------------
def log_uploaded(request_code, key):
    with open(UPLOADED_LOG, 'a', encoding='utf-8') as f:
        f.write(f"{datetime.now().isoformat()} | {request_code} | {key}\n")

# ---------------------------------------------------
def check_remote_control():
    global is_maintenance, last_status
    if not is_internet_available():
        return

    url = f"{API_BASE_URL}/api/v1/server-status"
    headers = {
        "Authorization": f"Bearer {API_SECRET_TOKEN}",
        "Content-Type": "application/json"
    }

    try:
        response = requests.get(url, headers=headers, timeout=12)
        if response.status_code == 200 and response.json():
            current_status = str(response.json().get("status", "")).strip()
            
            if current_status == "0" and last_status != "0":
                is_maintenance = True
                with open(MAINTENANCE_FILE, 'w', encoding='utf-8') as f:
                    f.write(f'STATUS="0" → MAINTENANCE MODE at {datetime.now().isoformat()}\n')
                print('[MAINTENANCE] status = "0" → Key generation DISABLED (Server running)')
            
            elif current_status == "1" and last_status != "1":
                is_maintenance = False
                if os.path.exists(MAINTENANCE_FILE):
                    try:
                        os.remove(MAINTENANCE_FILE)
                    except:
                        pass
                print('[MAINTENANCE] status = "1" → Key generation ENABLED')
            
            last_status = current_status
    except requests.exceptions.Timeout:
        print("[REMOTE CHECK] Timeout. Will retry later.")
    except Exception as e:
        print(f"[REMOTE CHECK] Error: {e}")

# ---------------------------------------------------
def background_controller():
    while True:
        time.sleep(300)
        if is_internet_available():
            check_remote_control()

# ---------------------------------------------------
def background_uploader():
    while True:
        time.sleep(5)

        if is_maintenance or os.path.exists(MAINTENANCE_FILE):
            print("[UPLOAD] Skipped: Maintenance mode active.")
            time.sleep(60)
            continue

        if not is_internet_available():
            print("[Background] No internet. Skipping upload...")
            continue

        pending = load_pending_uploads()
        if not pending:
            continue

        print(f"[Background] Found {len(pending)} pending records. Uploading...")
        api_records = []
        for item in pending:
            api_records.append({
                "request_code": item["request_code"],
                "activation_key": item["activation_key"],
                "generated_at": item["timestamp"],
                "device_ip": item.get("ip", "unknown")
            })

        if upload_to_server(api_records):
            for item in pending:
                log_uploaded(item["request_code"], item["activation_key"])
            save_pending_uploads([])
            print("[Background] All pending data uploaded and cleared.")
        else:
            print("[Background] Upload failed. Will retry later.")

# ---------------------------------------------------
def generate_activation_key(request_code, app_type="Restaurant"):
    secret_key = APP_SECRETS.get(app_type, APP_SECRETS["Restaurant"])

    data_to_hash = f"{request_code}::{secret_key}"
    hasher = hashlib.sha256()
    hasher.update(data_to_hash.encode('utf-8'))
    full_hash = hasher.hexdigest().upper()
    key = full_hash[:16]
    return "-".join(key[i:i+4] for i in range(0, 16, 4))

# ---------------------------------------------------
def save_key_to_files(request_code, activation_key, client_ip="unknown"):
    if is_maintenance or os.path.exists(MAINTENANCE_FILE):
        raise IOError("Maintenance mode: Key generation is disabled.")

    data_line = f"REQUEST_CODE: {request_code}, ACTIVATION_KEY: {activation_key}\n"
    paths_to_save = [SAVE_PATH_1, SAVE_PATH_2]

    for file_path in paths_to_save:
        try:
            dir_name = os.path.dirname(file_path)
            os.makedirs(dir_name, exist_ok=True)
            with open(file_path, 'a', encoding='utf-8') as f:
                f.write(data_line)
        except Exception as e:
            print(f"CRITICAL ERROR: Failed to save key to {file_path}", file=sys.stderr)
            raise IOError(f"Failed to write to {file_path}")

    pending = load_pending_uploads()
    pending.append({
        "request_code": request_code,
        "activation_key": activation_key,
        "timestamp": datetime.now().isoformat(),
        "ip": client_ip
    })
    save_pending_uploads(pending)

# ---------------------------------------------------
@app.route('/generate_key', methods=['POST'])
def api_generate_key():
    global API_BASE_URL

    if is_maintenance or os.path.exists(MAINTENANCE_FILE):
        return jsonify({
            "error": "Server is under maintenance. Key generation is temporarily disabled.",
            "status": "maintenance_mode",
            "retry_after": "Check back later or contact admin."
        }), 503

    print("Received new key generation request...")
    data = request.json

    if not data or 'request_code' not in data:
        return jsonify({"error": "Missing 'request_code' in JSON body"}), 400

    requested_server_url = str(data.get('server_url', '')).strip().rstrip('/')
    if requested_server_url:
        if not requested_server_url.startswith(('http://', 'https://')):
            return jsonify({"error": "server_url must use http:// or https://"}), 400
        API_BASE_URL = requested_server_url

    request_code = data['request_code'].strip().upper()

    # جلب نوع البرنامج من الطلب (إذا لم يتم إرساله، سيعتبره 'Restaurant' كافتراضي)
    app_type = data.get('app_type', 'Restaurant')

    client_ip = request.remote_addr

    if len(request_code) != 14 or request_code[4] != '-' or request_code[9] != '-':
        return jsonify({"error": "Invalid request_code format. Expected 'XXXX-XXXX-XXXX'."}), 400

    try:
        # تمرير نوع البرنامج إلى دالة التوليد لضمان استخدام الكلمة السرية الصحيحة
        activation_key = generate_activation_key(request_code, app_type)

        save_key_to_files(request_code, activation_key, client_ip)

        return jsonify({
            "request_code": request_code,
            "activation_key": activation_key,
            "app_type": app_type,  # إرجاع نوع البرنامج في الرد للتأكيد
            "status": "generated_and_queued"
        }), 200

    except IOError as e:
        print(f"IOError during save: {e}", file=sys.stderr)
        return jsonify({"error": "Key generation failed (maintenance mode?)."}), 503
    except Exception as e:
        print(f"General error: {e}", file=sys.stderr)
        return jsonify({"error": "Unexpected error"}), 500

# ---------------------------------------------------
if __name__ == "__main__":
    print("--- Key Generation API Server ---")
    print(f"Primary log: {SAVE_PATH_1}")
    print(f"Backup + Queue: {SAVE_PATH_2}")
    print(f"Pending uploads: {PENDING_UPLOADS}")
    print(f"Cloud API: {API_BASE_URL} (Checking status & uploading logs)")

    uploader_thread = threading.Thread(target=background_uploader, daemon=True)
    controller_thread = threading.Thread(target=background_controller, daemon=True)
    uploader_thread.start()
    controller_thread.start()

    try:
        app.run(host='127.0.0.1', port=45632, debug=False, use_reloader=False)
    except KeyboardInterrupt:
        print("\n[SHUTDOWN] Server stopped by user.")
