# Activateur RMS

Activation key management with a native Rust desktop application, a compact
Rust Windows service, and a PostgreSQL-backed cloud API.

## Architecture

```text
ActivateurRMS.exe (native Rust UI and setup)
        |
        | installs/updates through NSSM
        | POST http://127.0.0.1:45632/generate_key
        v
KeyGenService.exe (Rust local service)
        |
        | /api/v1/server-status and /api/v1/activation-logs
        v
server.py (cloud API) ---> PostgreSQL keygen_restaurant
```

The client executables never receive PostgreSQL credentials. PostgreSQL is
accessed only by the hosted API over SSL-configured database connections.

## Project Layout

```text
ActivateurRMS/
|-- Cargo.toml                       Rust client workspace
|-- ActivateurRust/                  Native Windows UI/setup source
|-- KeyGenCommonRust/                Shared .env configuration parser
|-- KeyGenServiceRust/               Local service source
|-- ActivateurRMS.exe                Packaged native UI executable
|-- KeyGenService/KeyGenService.exe  Packaged service executable
|-- nssm/nssm.exe                    Windows service wrapper
|-- server.py                        PostgreSQL cloud API/dashboard
|-- server_requirements.txt          Cloud API Python dependencies
|-- .env.example                     Cloud server configuration example
`-- client.env.example               Desktop package configuration example
```

The Python cloud process is intentionally server-side only. The distributed
desktop interface and its local backend are both Rust executables.

## Build The Rust Client

```powershell
cd .\ActivateurRMS
cargo test
cargo clippy --all-targets -- -D warnings
cargo build --release
Copy-Item .\target\release\ActivateurRMS.exe .\ActivateurRMS.exe
Copy-Item .\target\release\KeyGenService.exe .\KeyGenService\KeyGenService.exe
```

The shared release profile optimizes for small Windows binaries with LTO,
symbol stripping, size optimization, and abort-on-panic behavior.

## Package And Install

Distribute these runtime assets together:

```text
ActivateurRMS.exe
KeyGenService\KeyGenService.exe
nssm\nssm.exe
.env
```

Create the distributed `.env` from `client.env.example`; it contains the API
URL and API token, never PostgreSQL credentials. `ActivateurRMS.exe` reads it
from its own folder, preloads the setup fields, and writes a filtered `.env`
beside the installed Rust service.

Start `ActivateurRMS.exe`, enter the cloud API URL and API token, then choose
`Installer / Mettre a jour`. The app checks API access, copies the local Rust
service into `%ProgramFiles%\KeyGenRMS`, installs or upgrades it through NSSM,
sets its environment, and starts it automatically. Administrative elevation
is required only for installation and updates.

## Run The Cloud API

Install dependencies on the server:

```powershell
python -m pip install -r .\server_requirements.txt
```

Set secrets outside git, then start the API:

```powershell
Copy-Item .\.env.example .\.env
# Edit .env with the PostgreSQL password and API token on the server only.
python .\server.py
```

The default database endpoint is
`sw4.duckdns.org:9005/keygen_restaurant` with `PGSSLMODE=require`. Override
the `PG*` variables or use `DATABASE_URL` where needed; see `.env.example`.

For an existing SQLite database, import it once before service use:

```powershell
python .\server.py --migrate-sqlite .\cloud_database.db
```

The import is idempotent by activation log id.

## Security

Do not commit `.env` files, PostgreSQL passwords, or API tokens. PostgreSQL
credentials belong only on the cloud server. Use an HTTPS cloud API endpoint
in production because the local service sends its bearer token and activation
records to that endpoint.
