# keygen_app.py
import os
import sys
import shutil
import subprocess
import requests
import flet as ft
import logging

SERVICE_NAME = "KeyGenService"
APP_PATH = r"C:\KeyGenService\KeyGenService.exe"
APP_DIR = r"C:\KeyGenService"
NSSM_DIR = r"C:\nssm"
NSSM_PATH = os.path.join(NSSM_DIR, "nssm.exe")

DEFAULT_SERVER_URL = os.environ.get(
    "KEYGEN_CLOUD_API_URL", "http://qylad-server.duckdns.org:7002"
).rstrip("/")
API_SECRET_TOKEN = os.environ.get("KEYGEN_API_SECRET_TOKEN", "")

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
    try:
        shutil.copytree(src, dst, dirs_exist_ok=True)
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
    logging.info("Vérification de la connexion Internet...")
    try:
        requests.get("https://www.google.com", timeout=5)
        logging.info("Connexion Internet OK.")
        return True
    except Exception as e:
        logging.error(f"Échec de la connexion Internet: {e}")
        return False

def is_server_enabled():
    logging.info("Vérification de l'état du serveur cloud...")
    if not API_SECRET_TOKEN:
        logging.error("Variable KEYGEN_API_SECRET_TOKEN manquante.")
        return False

    if not is_internet_available():
        logging.warning("Impossible de vérifier: pas d'Internet.")
        return False

    try:
        url = f"{DEFAULT_SERVER_URL}/api/v1/server-status"
        headers = {
            "Authorization": f"Bearer {API_SECRET_TOKEN}",
            "Content-Type": "application/json"
        }

        logging.info(f"Tentative de connexion à: {url}")
        response = requests.get(url, headers=headers, timeout=10)
        logging.info(f"Réponse du serveur (Code HTTP): {response.status_code}")

        if response.status_code == 200:
            status = str(response.json().get("status", "")).strip()
            logging.info(f"Statut exact reçu du serveur: '{status}'")
            return status == "1"

        elif response.status_code in [401, 403]:
            logging.error(f"Refusé (Token invalide ou manquant). Réponse: {response.text}")
            return False
        else:
            logging.warning(f"Réponse inattendue: {response.text}")
            return False

    except requests.exceptions.ConnectionError:
        logging.error("Erreur de connexion: Impossible de joindre le domaine DuckDNS (Problème de réseau ou NAT Loopback).")
        return False
    except Exception as e:
        logging.error(f"Erreur inattendue lors de la vérification: {e}")
        return False

def install_backend():
    logging.info("Début de l'installation du backend...")

    if service_exists():
        logging.info("Le service existe déjà. Mise à jour vers le backend Rust.")

    if not is_internet_available():
        logging.error("Installation annulée: pas d'Internet.")
        return False

    status = is_server_enabled()
    if not status:
        logging.warning("Le serveur distant est désactivé, mais l'installation continue pour test.")
        return False

    # 2. طلب صلاحيات المسؤول
    run_as_admin()

    # 3. التأكد من وجود المجلدات بالأسماء الصحيحة محلياً قبل النسخ
    SOURCE_APP_DIR = "KeyGenService" # تم التصحيح ليتطابق مع مجلدك

    if not os.path.exists(SOURCE_APP_DIR):
        logging.error(f"Échec: Dossier source introuvable: '{SOURCE_APP_DIR}'")
        return False

    if not os.path.exists("nssm"):
        logging.error("Échec: Dossier source introuvable: 'nssm'")
        return False

    logging.info("Copie des dossiers nécessaires vers C:\\ ...")
    if not copy_folder(SOURCE_APP_DIR, APP_DIR):
        logging.error(f"Échec de la copie du dossier {SOURCE_APP_DIR}.")
        return False

    if not copy_folder("nssm", NSSM_DIR):
        logging.error("Échec de la copie du dossier nssm.")
        return False

    # 4. التأكد من وصول الملفات إلى القرص C
    if not os.path.isfile(APP_PATH):
        logging.error(f"Fichier exécutable introuvable après copie: {APP_PATH}")
        return False
    if not os.path.isfile(NSSM_PATH):
        logging.error(f"Exécutable NSSM introuvable après copie: {NSSM_PATH}")
        return False

    # 5. تثبيت وتشغيل الخدمة
    logging.info("Configuration du service avec NSSM...")
    run_cmd([NSSM_PATH, "remove", SERVICE_NAME, "confirm"])
    run_cmd([NSSM_PATH, "install", SERVICE_NAME, APP_PATH])
    run_cmd([NSSM_PATH, "set", SERVICE_NAME, "AppDirectory", APP_DIR])
    run_cmd([
        NSSM_PATH, "set", SERVICE_NAME, "AppEnvironmentExtra",
        f"KEYGEN_CLOUD_API_URL={DEFAULT_SERVER_URL}",
        f"KEYGEN_API_SECRET_TOKEN={API_SECRET_TOKEN}"
    ])
    run_cmd([NSSM_PATH, "set", SERVICE_NAME, "Start", "SERVICE_AUTO_START"])

    start_result = run_cmd(["net", "start", SERVICE_NAME])

    if start_result:
        logging.info("Installation du backend terminée et service démarré avec succès!")
        return True
    else:
        logging.error("Le service a été installé mais n'a pas pu démarrer automatiquement.")
        return False

def hide_console():
    if sys.platform.startswith("win"):
        try:
            import ctypes
            ctypes.windll.user32.ShowWindow(ctypes.windll.kernel32.GetConsoleWindow(), 0)
        except:
            pass

def prelaunch_setup():
    install_backend()

# ==========================================
# واجهة المستخدم (Flet)
# ==========================================
LOCAL_API_URL = "http://127.0.0.1:45632/generate_key"

def main(page: ft.Page):
    page.title = "Générateur de clés (Multi-Logiciels)"
    page.window.width = 450
    page.window.height = 600
    page.window_resizable = False
    page.bgcolor = ft.Colors.BLUE_GREY_50
    page.vertical_alignment = ft.MainAxisAlignment.CENTER
    page.horizontal_alignment = ft.CrossAxisAlignment.CENTER

    # حقل رابط السيرفر
    txt_server_ip = ft.TextField(
        label="URL du Serveur API",
        value=DEFAULT_SERVER_URL,
        width=350,
        border_radius=10,
        prefix_icon=ft.Icons.CLOUD,
    )

    # قائمة اختيار البرنامج
    dropdown_app = ft.Dropdown(
        label="Choisissez le logiciel",
        width=350,
        border_radius=10,
        prefix_icon=ft.Icons.APPS,
        options=[
            ft.dropdown.Option(key="Restaurant", text="تسيير المطاعم (Restaurant)"),
            ft.dropdown.Option(key="Lab", text="تسيير مخزون المخابر (Laboratoire)"),
            ft.dropdown.Option(key="Jewelry", text="تسيير محلات الذهب (Bijouterie)"),
        ],
        value="Restaurant" # القيمة الافتراضية
    )

    txt_request_code = ft.TextField(
        label="Code de demande (Request Code)",
        hint_text="Entrez le code (ex: F81A-67A7-C6AA)",
        width=350,
        border_radius=10,
        prefix_icon=ft.Icons.FINGERPRINT,
    )

    txt_key_output = ft.TextField(
        label="Clé d'activation générée",
        read_only=True,
        width=350,
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
        app_type = dropdown_app.value
        server_ip = txt_server_ip.value.strip()

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

        # إرسال البيانات (بما فيها نوع البرنامج والسيرفر) إلى الخدمة المحلية
        payload = {
            "request_code": code,
            "app_type": app_type,
            "server_url": server_ip
        }

        try:
            response = requests.post(LOCAL_API_URL, json=payload, timeout=10)
            if response.status_code == 200:
                txt_key_output.value = response.json().get("activation_key", "")
            elif response.status_code == 503:
                txt_request_code.error_text = "Serveur en maintenance"
            else:
                txt_request_code.error_text = response.json().get("error", f"Erreur {response.status_code}")
        except requests.ConnectionError:
            txt_request_code.error_text = "Service local arrêté. Redémarrez l'app."
        except requests.Timeout:
            txt_request_code.error_text = "Délai dépassé"
        except Exception as ex:
            txt_request_code.error_text = f"Erreur inconnue: {ex}"

        btn_generate.text = "Générer la clé"
        btn_generate.disabled = False
        page.update()

    btn_generate = ft.ElevatedButton(
        text="Générer la clé",
        icon=ft.Icons.VPN_KEY_ROUNDED,
        width=350,
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
            width=410,
            padding=30,
            border_radius=15,
            bgcolor=ft.Colors.WHITE,
            shadow=ft.BoxShadow(spread_radius=1, blur_radius=15, color=ft.Colors.BLUE_GREY_100, offset=ft.Offset(0, 5)),
            content=ft.Column([
                ft.Text("Générateur Multi-Logiciels", size=22, weight=ft.FontWeight.BOLD, color=ft.Colors.BLUE_GREY_800),
                ft.Divider(height=15, color=ft.Colors.TRANSPARENT),
                txt_server_ip,
                dropdown_app,
                txt_request_code,
                btn_generate,
                txt_key_output,
            ], horizontal_alignment=ft.CrossAxisAlignment.CENTER, spacing=15)
        )
    )

if __name__ == "__main__":
    hide_console()
    prelaunch_setup()
    ft.app(target=main, view=ft.AppView.FLET_APP)
