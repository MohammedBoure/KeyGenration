import os
from pathlib import Path

from dotenv import load_dotenv


SERVICE_ROOT = Path(__file__).resolve().parent.parent


def load_environment():
    load_dotenv(SERVICE_ROOT / ".env")


def debug_enabled():
    return os.environ.get("FLASK_DEBUG", "0") == "1"
