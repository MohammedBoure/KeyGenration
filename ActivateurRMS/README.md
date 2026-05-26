# Activateur RMS 2025 - Restaurant Management System Key Generator

**The next generation of activation generators for Restaurant Management Systems.**
A complete system: Elegant UI + Local backend server + Instant remote control via Supabase.

[![Python 3.9+](https://img.shields.io/badge/Python-3.9%2B-blue)](https://python.org)
[![Flet UI](https://img.shields.io/badge/Flet-Modern_UI-orange)](https://flet.dev)
[![Supabase](https://img.shields.io/badge/Supabase-Remote_Control-green)](https://supabase.com)
[![Status](https://img.shields.io/badge/Status-Active_2025-success)](https://github.com)

## Final Project Structure (As it should be)

```
ActivateurRMS/
├── ActivateurRMS.py              Main interface + Auto-installation (Flet)
├── installer.py                  Manual service installation/uninstallation tool (Optional)
├── icon.png                      Official icon (282 KB)
├── nssm/                         NSSM 64-bit tool (To convert the program to a service)
├── KeyGenService/                Example bundled version (For demonstration only)
│   └── KeyGenService.exe         Dummy file (0 bytes) - To illustrate the final form
└── KeyGenServiceSC/              Actual source code (This is where the server is compiled)
├── KeyGenService.py              Original code (Flask API + Supabase)
└── README.md                     Internal server explanation
```
## Complete Build Process (Step-by-step) - For Developers and Distributors

### 1. Building the Backend Server (KeyGenService.exe)

```bash
cd KeyGenServiceSC

pyinstaller --onedir --noconsole --icon=../icon.png --name "KeyGenService" KeyGenService.py

```
Output: dist\KeyGenService
Place the entire folder in: KeyGenService\

### 2. Building the Main Interface (Activateur RMS.exe)

```bash
# From the root directory
pyinstaller --onefile --windowed --noconsole --icon=icon.png --name "Activateur RMS" ActivateurRMS.py
```

### 3. (Optional) Manual Usage of installer.py

```bash
python installer.py
```

### How it works for the client:

1.  `Activateur RMS.exe` is opened.
2.  Internet and server status (Supabase) are checked.
3.  If the system is activated (`status = "1"`):
    *   Everything is copied to `C:\keygen_exe`.
    *   `KeyGenService.exe` is installed as a Windows service (using NSSM).
    *   The elegant generation interface is launched.

Every key generated is immediately uploaded to Supabase.

### Full Remote Control

**Supabase Table:** `server_control`
**Row:** `id = 1`
**Field:** `status`
*   `"1"` → Activated (Generates keys)
*   `"0"` → Immediately deactivated (globally)

## PostgreSQL-backed API Server

`server.py` stores the control status and activation logs in PostgreSQL. It
defaults to `sw4.duckdns.org:9005/keygen_restaurant` and requires SSL.
Secrets must be supplied as environment variables and must not be committed.

Install the server dependencies:

```powershell
python -m pip install -r .\server_requirements.txt
```

Set configuration before running the API server. See `.env.example` for all
available variables.

```powershell
$env:PGPASSWORD = "<database-password>"
$env:KEYGEN_API_SECRET_TOKEN = "<api-token>"
python .\server.py
```

For an existing local `cloud_database.db`, run the one-time idempotent import
before starting the web server:

```powershell
$env:PGPASSWORD = "<database-password>"
$env:KEYGEN_API_SECRET_TOKEN = "<api-token>"
python .\server.py --migrate-sqlite .\cloud_database.db
```
