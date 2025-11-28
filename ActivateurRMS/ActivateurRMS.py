# keygen_app.py
import os
import sys
import shutil
import subprocess
import requests
from datetime import datetime

SERVICE_NAME = "KeyGenService"
APP_PATH = r"C:\KeyGenService\KeyGenService.exe"
APP_DIR = r"C:\KeyGenService"
NSSM_DIR = r"C:\nssm"
NSSM_PATH = os.path.join(NSSM_DIR, "nssm.exe")

# Supabase Control
SUPABASE_URL = "https://xlmphvxehdomywigsrhq.supabase.co"
SUPABASE_API_KEY = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJzdXBhYmFzZSIsInJlZiI6InhsbXBodnhlaGRvbXl3aWdzcmhxIiwicm9sZSI6InNlcnZpY2Vfcm9sZSIsImlhdCI6MTc1NDU1OTM1NiwiZXhwIjoyMDcwMTM1MzU2fQ.pYm02fArrbUc8UjmiFb3Qrc4z3RntGrM-DEmMdhPyGA"
CONTROL_TABLE = "server_control"

def run_cmd(cmd):
    try:
        subprocess.run(cmd, check=True, capture_output=True, shell=isinstance(cmd, str))
        return True
    except:
        return False

def service_exists():
    try:
        result = subprocess.run(["sc", "query", SERVICE_NAME], capture_output=True, text=True)
        return "1060" not in result.stdout
    except:
        return False

def copy_folder(src, dst):
    if os.path.exists(dst):
        return True
    try:
        shutil.copytree(src, dst)
        return True
    except:
        return False

def is_admin():
    try:
        import ctypes
        return ctypes.windll.shell32.IsUserAnAdmin() != 0
    except:
        return False

def run_as_admin():
    if not is_admin():
        try:
            import ctypes
            ctypes.windll.shell32.ShellExecuteW(None, "runas", sys.executable, f'"{__file__}"', None, 1)
            sys.exit(0)
        except:
            sys.exit(1)

def is_internet_available():
    try:
        requests.get("https://httpbin.org/ip", timeout=5)
        return True
    except:
        return False

def is_server_enabled():
    if not is_internet_available():
        return False
    try:
        url = f"{SUPABASE_URL}/rest/v1/{CONTROL_TABLE}?select=status&id=eq.1"
        headers = {
            "apikey": SUPABASE_API_KEY,
            "Authorization": f"Bearer {SUPABASE_API_KEY}"
        }
        response = requests.get(url, headers=headers, timeout=10)
        if response.status_code == 200 and response.json():
            status = response.json()[0].get("status", "").strip()
            return status == "1"
        return False
    except:
        return False

def install_backend():
    if service_exists():
        return True

    if not is_internet_available():
        return False

    if not is_server_enabled():
        return False 

    run_as_admin()

    if not copy_folder("keygen_exe", APP_DIR):
        return False
    if not copy_folder("nssm", NSSM_DIR):
        return False

    if not os.path.isfile(APP_PATH):
        return False
    if not os.path.isfile(NSSM_PATH):
        return False

    run_cmd([NSSM_PATH, "remove", SERVICE_NAME, "confirm"])
    run_cmd([NSSM_PATH, "install", SERVICE_NAME, APP_PATH])
    run_cmd([NSSM_PATH, "set", SERVICE_NAME, "AppDirectory", APP_DIR])
    run_cmd([NSSM_PATH, "set", SERVICE_NAME, "Start", "SERVICE_AUTO_START"])
    run_cmd(["net", "start", SERVICE_NAME])
    return True

def hide_console():
    if sys.platform.startswith("win"):
        try:
            import ctypes
            ctypes.windll.user32.ShowWindow(ctypes.windll.kernel32.GetConsoleWindow(), 0)
        except:
            pass

def prelaunch_setup():
    install_backend()

import flet as ft
import requests

API_URL = "http://127.0.0.1:45632/generate_key"

def main(page: ft.Page):
    page.title = "Générateur de clés d'activation"
    page.window.width = 400
    page.window.height = 420
    page.window_resizable = False
    page.bgcolor = ft.Colors.BLUE_GREY_50
    page.vertical_alignment = ft.MainAxisAlignment.CENTER
    page.horizontal_alignment = ft.CrossAxisAlignment.CENTER

    txt_request_code = ft.TextField(
        label="Code de demande (Request Code)",
        hint_text="Entrez le code (ex: F81A-67A7-C6AA)",
        width=320,
        border_radius=10,
        prefix_icon=ft.Icons.FINGERPRINT,
    )

    txt_key_output = ft.TextField(
        label="Clé d'activation générée",
        read_only=True,
        width=320,
        border_radius=10,
        hint_text="La clé apparaîtra ici...",
        prefix=ft.IconButton(
            icon=ft.Icons.COPY,
            tooltip="Copier la clé",
            on_click=lambda _: page.set_clipboard(txt_key_output.value) if txt_key_output.value else None,
        )
    )

    def generate_key(e):
        code = txt_request_code.value.strip().upper()
        if not code:
            txt_request_code.error_text = "Veuillez entrer le code"
            page.update()
            return
        if len(code) != 14 or code[4] != '-' or code[9] != '-':
            txt_request_code.error_text = "Format: XXXX-XXXX-XXXX"
            page.update()
            return

        txt_request_code.error_text = None
        txt_key_output.value = ""
        btn_generate.text = "Génération..."
        btn_generate.disabled = True
        page.update()

        try:
            response = requests.post(API_URL, json={"request_code": code}, timeout=10)
            if response.status_code == 200:
                txt_key_output.value = response.json().get("activation_key", "")
            elif response.status_code == 503:
                txt_request_code.error_text = "Serveur en maintenance"
            else:
                txt_request_code.error_text = response.json().get("error", f"Erreur {response.status_code}")
        except requests.ConnectionError:
            txt_request_code.error_text = "API non lancée. Redémarrez l'app."
        except requests.Timeout:
            txt_request_code.error_text = "Délai dépassé"
        except Exception:
            txt_request_code.error_text = "Erreur inconnue"

        btn_generate.text = "Générer la clé"
        btn_generate.disabled = False
        page.update()

    btn_generate = ft.ElevatedButton(
        text="Générer la clé",
        icon=ft.Icons.VPN_KEY_ROUNDED,
        width=320,
        height=50,
        on_click=generate_key,
        style=ft.ButtonStyle(
            shape=ft.RoundedRectangleBorder(radius=10),
            bgcolor=ft.Colors.BLUE_600,
            color=ft.Colors.WHITE,
        )
    )

    page.add(
        ft.Container(
            width=380,
            padding=30,
            border_radius=15,
            bgcolor=ft.Colors.WHITE,
            shadow=ft.BoxShadow(spread_radius=1, blur_radius=15, color=ft.Colors.BLUE_GREY_100, offset=ft.Offset(0, 5)),
            content=ft.Column([
                ft.Text("RestaurantManagement", size=26, weight=ft.FontWeight.BOLD, color=ft.Colors.BLUE_GREY_800),
                ft.Divider(height=25, color=ft.Colors.TRANSPARENT),
                txt_request_code,
                btn_generate,
                txt_key_output,
            ], horizontal_alignment=ft.CrossAxisAlignment.CENTER, spacing=20)
        )
    )

if __name__ == "__main__":
    hide_console()
    prelaunch_setup()
    ft.app(target=main, view=ft.AppView.FLET_APP)