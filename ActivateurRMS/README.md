# Activateur RMS

Activation key management with a Flet user interface, a small local Rust
service, and a PostgreSQL-backed cloud API.

## Architecture

```text
Activateur.py (Flet UI)
        |
        | POST http://127.0.0.1:45632/generate_key
        v
KeyGenServiceRust / KeyGenService.exe
        |
        | /api/v1/server-status and /api/v1/activation-logs
        v
server.py (cloud API) ---> PostgreSQL keygen_restaurant
```

The desktop executable never receives PostgreSQL credentials. PostgreSQL is
accessed only by the cloud API, using SSL and environment variables.

## Project Layout

```text
ActivateurRMS/
|-- Activateur.py                 Flet interface and Windows service setup
|-- installer.py                  Manual NSSM service installer/upgrader
|-- server.py                     PostgreSQL-backed cloud API and dashboard
|-- server_requirements.txt       Cloud API Python dependencies
|-- KeyGenServiceRust/            Preferred local backend source
|-- KeyGenService/KeyGenService.exe  Packaged local service executable
|-- KeyGenServiceSC/              Legacy Python backend source
`-- nssm/nssm.exe                Windows service wrapper
```

## Build The Rust Backend

Rust replaces the PyInstaller backend executable. It implements the same
`POST /generate_key` endpoint used by the interface, including local queueing,
remote maintenance status, and cloud log upload.

```powershell
cd .\KeyGenServiceRust
cargo test
cargo build --release
Copy-Item .\target\release\KeyGenService.exe ..\KeyGenService\KeyGenService.exe
```

The release profile enables size-oriented optimization, LTO, symbol stripping,
and abort-on-panic so the distributed executable remains compact.

## Run The Cloud API

Install dependencies:

```powershell
python -m pip install -r .\server_requirements.txt
```

Set secrets outside git, then start the API:

```powershell
$env:PGPASSWORD = "<database-password>"
$env:KEYGEN_API_SECRET_TOKEN = "<api-token>"
python .\server.py
```

The default database endpoint is
`sw4.duckdns.org:9005/keygen_restaurant` with `PGSSLMODE=require`. Override
the `PG*` variables or use `DATABASE_URL` where needed; see `.env.example`.

For an existing SQLite database, import it once before starting service use:

```powershell
$env:PGPASSWORD = "<database-password>"
python .\server.py --migrate-sqlite .\cloud_database.db
```

The import is idempotent by activation log id.

## Install The Local Service

The local service sends data to the cloud API backed by PostgreSQL:

```powershell
$env:KEYGEN_CLOUD_API_URL = "http://qylad-server.duckdns.org:7002"
$env:KEYGEN_API_SECRET_TOKEN = "<api-token>"
python .\installer.py
```

`installer.py` installs `KeyGenService\KeyGenService.exe` through NSSM and
stores the cloud API settings in that service environment.

## Legacy Backend

`KeyGenServiceSC\KeyGenService.py` remains available as a Python compatibility
implementation. It uses the same cloud API environment variables and no
longer connects to Supabase. New packages should use the Rust executable.

## Secrets

Do not commit `.env` files, PostgreSQL passwords, or API tokens. A desktop
service token should be scoped and rotated if it is distributed outside a
trusted environment.
