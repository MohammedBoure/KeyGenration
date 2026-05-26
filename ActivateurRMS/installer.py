import os
import shutil
import subprocess
import webbrowser
import atexit
import signal
import sys
import logging
from logging.handlers import RotatingFileHandler

# ---------------- Logging setup ----------------
LOG_FILENAME = os.path.join(os.path.dirname(os.path.abspath(__file__)), "installer.log")
logger = logging.getLogger("installer")
logger.setLevel(logging.DEBUG)

fh = RotatingFileHandler(LOG_FILENAME, maxBytes=5*1024*1024, backupCount=3, encoding="utf-8")
fh.setLevel(logging.DEBUG)
formatter = logging.Formatter("%(asctime)s [%(levelname)s] %(message)s")
fh.setFormatter(formatter)
logger.addHandler(fh)

ch = logging.StreamHandler(sys.stdout)
ch.setLevel(logging.INFO)
ch.setFormatter(formatter)
logger.addHandler(ch)

def log_and_print(msg, level="info"):
    if level == "debug":
        logger.debug(msg)
    elif level == "warning":
        logger.warning(msg)
    elif level == "error":
        logger.error(msg)
    else:
        logger.info(msg)

# ====== Relaunch as admin on Windows (UAC) ======
def is_windows_admin():
    if os.name != "nt":
        return True
    try:
        import ctypes
        return ctypes.windll.shell32.IsUserAnAdmin() != 0
    except:
        return False

def relaunch_as_admin():
    if os.name != "nt":
        return False
    try:
        import ctypes
        params = " ".join([f'"{arg}"' for arg in sys.argv[1:]])
        script = sys.argv[0]
        cmd = f'"{script}" {params}' if params else f'"{script}"'
        logger.info("Requesting UAC elevation...")
        ctypes.windll.shell32.ShellExecuteW(None, "runas", sys.executable, cmd, None, 1)
        return True
    except Exception as e:
        logger.error(f"Elevation failed: {e}")
        return False

if __name__ == "__main__" and os.name == "nt" and not is_windows_admin():
    logger.info("Requesting Administrator privileges...")
    if relaunch_as_admin():
        sys.exit(0)
    else:
        logger.error("Please run as Administrator.")
        sys.exit(1)

# ====== إعدادات التطبيق ======
SERVICE_NAME = "KeyGenService"
APP_PATH = r"C:\KeyGenService\KeyGenService.exe"
APP_DIR = r"C:\KeyGenService"
NSSM_DIR = r"C:\nssm"
NSSM_PATH = os.path.join(NSSM_DIR, "nssm.exe")
SOURCE_APP_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), "KeyGenService")
CLOUD_API_URL = os.environ.get(
    "KEYGEN_CLOUD_API_URL", "http://qylad-server.duckdns.org:7002"
).rstrip("/")
API_SECRET_TOKEN = os.environ.get("KEYGEN_API_SECRET_TOKEN", "")

# ====== دالة تشغيل الأوامر ======
def run_cmd(cmd, description=None):
    try:
        if description:
            logger.info(f"{description} ...")
        result = subprocess.run(
            cmd,
            check=True,
            capture_output=True,
            text=True,
            shell=True if isinstance(cmd, str) else False,
        )
        if result.stdout.strip():
            logger.info(result.stdout.strip())
        if result.stderr.strip():
            logger.warning(result.stderr.strip())
        return True
    except subprocess.CalledProcessError as e:
        logger.error(f"Failed: {description or cmd}")
        logger.error(e.stderr or str(e))
        return False

# ====== التحقق من وجود الخدمة ======
def service_exists(name):
    try:
        result = subprocess.run(["sc", "query", name], capture_output=True, text=True)
        return "1060" not in result.stdout
    except:
        return False

# ====== نسخ المجلدات ======
def copy_folder(src, dst):
    if os.path.exists(dst):
        logger.info(f"Updating existing folder: {dst}")
        shutil.copytree(src, dst, dirs_exist_ok=True)
        return
    logger.info(f"Copying {src} to {dst}")
    shutil.copytree(src, dst)

# ====== تثبيت الخدمة ======
def install_service():
    logger.info("=== Installing KeyGenService ===")

    if not API_SECRET_TOKEN:
        logger.error("Set KEYGEN_API_SECRET_TOKEN before installing the service.")
        return False

    # نسخ الملفات
    copy_folder(SOURCE_APP_DIR, APP_DIR)
    copy_folder("nssm", NSSM_DIR)

    if not os.path.isfile(APP_PATH):
        logger.error(f"KeyGenService.exe not found: {APP_PATH}")
        return False

    if not os.path.isfile(NSSM_PATH):
        logger.error(f"nssm.exe not found: {NSSM_PATH}")
        return False

    # إزالة الخدمة إذا كانت موجودة
    if service_exists(SERVICE_NAME):
        run_cmd([NSSM_PATH, "remove", SERVICE_NAME, "confirm"], "Removing old service")

    # تثبيت الخدمة
    if not run_cmd([NSSM_PATH, "install", SERVICE_NAME, APP_PATH], "Installing service"):
        return False
    if not run_cmd([NSSM_PATH, "set", SERVICE_NAME, "AppDirectory", APP_DIR], "Setting AppDirectory"):
        return False
    if not run_cmd([
        NSSM_PATH, "set", SERVICE_NAME, "AppEnvironmentExtra",
        f"KEYGEN_CLOUD_API_URL={CLOUD_API_URL}",
        f"KEYGEN_API_SECRET_TOKEN={API_SECRET_TOKEN}"
    ], "Setting cloud API environment"):
        return False
    if not run_cmd([NSSM_PATH, "set", SERVICE_NAME, "Start", "SERVICE_AUTO_START"], "Setting auto-start"):
        return False

    # تشغيل الخدمة
    if not run_cmd(["net", "start", SERVICE_NAME], "Starting service"):
        return False

    run_cmd(["sc", "query", SERVICE_NAME], "Service status")

    logger.info("Service installed and started successfully!")
    logger.info(f"Service Name: {SERVICE_NAME}")
    logger.info(f"App Path: {APP_PATH}")

    print("\n[!] Service is now running in the background.")
    print("[!] Close this window to continue. The service will remain active.")
    return True

# ====== إزالة الخدمة ======
def remove_service():
    logger.info("=== Removing KeyGenService ===")

    if service_exists(SERVICE_NAME):
        run_cmd(["net", "stop", SERVICE_NAME], "Stopping service")
        run_cmd([NSSM_PATH, "remove", SERVICE_NAME, "confirm"], "Removing service")
        logger.info("Service removed.")
    else:
        logger.info("Service not found.")

    # حذف المجلدات (اختياري)
    confirm = input("Delete C:\\KeyGenService and C:\\nssm folders? (y/N): ").strip().lower()
    if confirm == 'y':
        for path in [APP_DIR, NSSM_DIR]:
            if os.path.exists(path):
                shutil.rmtree(path, ignore_errors=True)
                logger.info(f"Deleted: {path}")

    logger.info("Cleanup completed.")
    print("\nSUCCESS: Service and files removed.")
    input("\nPress Enter to exit...")

# ====== تنظيف عند الخروج (لا تُوقف الخدمة) ======
def cleanup():
    logger.info("Installer is closing. Service remains running.")

atexit.register(cleanup)

def signal_handler(sig, frame):
    logger.info("Interrupted. Closing installer...")
    cleanup()
    sys.exit(0)

# ====== الواجهة الرئيسية ======
def main():
    logger.info("=== KeyGen Service Manager ===")
    print("\n" + "="*50)
    print("     KeyGen Service Installer")
    print("="*50)
    print("1) Install & Run Service")
    print("2) Remove Service & Files")
    print("-"*50)
    
    try:
        choice = input("Choose (1 or 2): ").strip()
        if choice == "1":
            print("\nStarting installation...")
            success = install_service()
            if not success:
                print("\nInstallation failed. Check log for details.")
        elif choice == "2":
            print("\nStarting removal...")
            remove_service()
        else:
            print("Invalid choice! Please enter 1 or 2.")
            input("\nPress Enter to exit...")
            return
    except Exception as e:
        logger.error(f"Unexpected error in main: {e}")
        print(f"\nFATAL ERROR: {e}")
        input("\nPress Enter to close...")
        return

    # --- النهاية ---
    print("\n" + "="*50)
    print("     OPERATION COMPLETED")
    print("="*50)
    print(f"Log saved to: {LOG_FILENAME}")
    print("\nPress Enter to exit...")
    input()

if __name__ == "__main__":
    signal.signal(signal.SIGINT, signal_handler)
    signal.signal(signal.SIGTERM, signal_handler)
    main()
