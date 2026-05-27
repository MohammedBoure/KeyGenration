# KeyGenRestorant / Activateur RMS

Lightweight Windows activation-key generator for products configured through a
private `.env` file. It records generation activity in PostgreSQL and lets an
administrator enable or disable generation remotely.

The user interface and the permanent local backend are written in Rust. The
administration website is a local FastAPI application used on the
administrator's computer.

Documentation language: **English (primary)** | [Arabic documentation](docs/ar/README.md)

## Architecture

```text
Generator computer
  ActivateurRMS.exe (Rust UI)
       |
       | Local HTTP only: 127.0.0.1:45632
       v
  KeyGenService.exe (Rust Windows service installed through NSSM)
       |
       | PostgreSQL over SSL
       v
  Remote PostgreSQL database
       ^
       | PostgreSQL over SSL
       |
Administration computer
  Local FastAPI dashboard: http://127.0.0.1:8080/
```

There is no public cloud HTTP API between the generator and the database.
`KeyGenService.exe` connects directly to PostgreSQL to read the control status
and upload records. The FastAPI dashboard connects directly to the same
database to view records, delete an individual record when needed, and change
the control status.

## Components

| Component | Purpose |
| --- | --- |
| `ActivateurRMS.exe` | Native UI and Windows-service installation, update, repair, and removal |
| `KeyGenService.exe` | Local key generation, queue storage, PostgreSQL synchronization, and maintenance control |
| `nssm.exe` | Runs `KeyGenService` automatically as a Windows service |
| PostgreSQL | Stores `server_control.status` and `activation_logs` |
| Local FastAPI dashboard | Displays logs, deletes a selected log after two confirmations, and edits status `0/1` |

## Runtime Flow

1. The UI checks whether the Windows service named `KeyGenService` is registered.
2. A healthy service is accepted only when its backend contract and configured
   token-name list match the UI's local `.env`.
3. If an installed service is stopped, the UI attempts to start it.
4. If it is missing, obsolete, or broken, installation is allowed only when
   PostgreSQL is reachable and `server_control.status` is `1`.
5. After authorization, the UI requests Administrator permission and installs
   or repairs the service through NSSM.
6. On its first normal start, the service confirms status `1` and creates
   `%ProgramData%\KeyGenRMS\AUTHORIZED.txt`.
7. The UI displays the product names loaded from `.env`; the service generates
   a key locally using the selected private token, stores the operation
   locally, and uploads it when connectivity is available.
8. The service polls status `0/1`. Status `0` creates `MAINTENANCE.txt` and
   blocks new generation; status `1` enables generation again.

## Private Configuration

Put a configured `.env` beside `ActivateurRMS.exe` on a managed generator
computer only. It contains PostgreSQL credentials and all generation tokens:

```dotenv
PGHOST=replace-with-postgresql-host
PGPORT=replace-with-postgresql-port
PGDATABASE=replace-with-postgresql-database
PGUSER=replace-with-restricted-client-user
PGPASSWORD=replace-with-restricted-client-password
PGSSLMODE=require
PGCONNECT_TIMEOUT=10

KEYGEN_LISTEN_ADDRESS=127.0.0.1:45632
KEYGEN_STATUS_INTERVAL_SECONDS=15
KEYGEN_UPLOAD_INTERVAL_SECONDS=5

KEYGEN_TOKEN_IDS=restaurant,lab,jewelry
KEYGEN_TOKEN_RESTAURANT_NAME=Restaurant
KEYGEN_TOKEN_RESTAURANT_SECRET=replace-with-private-restaurant-token
KEYGEN_TOKEN_LAB_NAME=Lab
KEYGEN_TOKEN_LAB_SECRET=replace-with-private-lab-token
KEYGEN_TOKEN_JEWELRY_NAME=Jewelry
KEYGEN_TOKEN_JEWELRY_SECRET=replace-with-private-jewelry-token
```

To add a product, add an identifier to `KEYGEN_TOKEN_IDS`, define its
`KEYGEN_TOKEN_<ID>_NAME` and `KEYGEN_TOKEN_<ID>_SECRET`, then run:

```powershell
.\ActivateurRMS.exe --install
```

The executable files do not embed PostgreSQL connection details or private
generation tokens.

## Quick Start

### Windows Generator

The generated package has this layout:

```text
ActivateurRMS\
|-- ActivateurRMS.exe
|-- KeyGenService\KeyGenService.exe
|-- nssm\nssm.exe
`-- SHA256SUMS.txt
```

On the managed generator computer, place the private `.env` beside the UI and
install the local service:

```powershell
cd .\dist\windows\ActivateurRMS
.\ActivateurRMS.exe --install
.\ActivateurRMS.exe
```

Windows displays a UAC prompt when administrator permission is needed.

### Local Administration Dashboard

```powershell
cd .\services\cloud-api
python -m pip install -r .\requirements.txt
Copy-Item .\.env.example .\.env
# Edit .env with the dashboard PostgreSQL account.
python .\app.py --init-db-only
python .\fastapi_app.py
```

Open `http://127.0.0.1:8080/` on the same computer.

## Service Commands

Run these from the package folder containing `KeyGenService\` and `nssm\`:

| Command | Result |
| --- | --- |
| `.\ActivateurRMS.exe --install` | Installs, updates, or repairs the local backend service |
| `.\ActivateurRMS.exe --uninstall` | Stops and removes the backend service and installation files |
| `.\ActivateurRMS.exe --unstall` | Accepted alias for `--uninstall` |
| `.\ActivateurRMS.exe --help` | Displays command help |

Uninstall removes `%ProgramFiles%\KeyGenRMS` but preserves
`%ProgramData%\KeyGenRMS` so unsynchronized records are not lost.

## Local Data

| Location | Contents |
| --- | --- |
| Beside `ActivateurRMS.exe` | Initial private `.env` on the managed computer |
| `%ProgramFiles%\KeyGenRMS` | Installed service, NSSM, and service `.env` copy |
| `%ProgramData%\KeyGenRMS` | Generated-key logs, pending queue, authorization, and maintenance state |
| `services\cloud-api\.env` | Local dashboard configuration on the administration computer |

| Data file | Purpose |
| --- | --- |
| `generated_keys.txt` | Local generated-key log |
| `pending_uploads.json` | Records waiting for upload to PostgreSQL |
| `uploaded.log` | Successfully uploaded record trace |
| `AUTHORIZED.txt` | Initial authorization marker after receiving status `1` |
| `MAINTENANCE.txt` | Local disabled state after receiving status `0` |

## Build

```powershell
Copy-Item .\packaging\windows\client.env.example .\packaging\windows\.env
# Configure the private local .env; never add it to git.
cargo fmt --all
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
.\packaging\windows\package.cmd
```

The default output is `dist\windows\ActivateurRMS\`. Packaging removes any
old `.env` from the output directory; put the real `.env` on the managed
computer after packaging.

## Security Limits

- Use a restricted PostgreSQL account for generator computers: `SELECT` on
  `server_control` and `INSERT` on `activation_logs` only.
- Never distribute or commit `.env` files containing credentials or tokens.
- Offline generation requires a token to exist locally in the installed
  service configuration; a computer administrator can therefore extract it.
- A disconnected generator cannot receive a new status `0` until it reconnects.
- The FastAPI dashboard is local-only by default and must not be exposed
  publicly without additional authentication and transport protection.
- Any older secrets that existed in git history must be rotated before the
  repository is made public.

## Documentation

| Guide | Contents |
| --- | --- |
| [Documentation index](docs/README.md) | Starting point for English documentation |
| [Architecture and data flow](docs/en/architecture.md) | Component boundaries and runtime flows |
| [User guide](docs/en/usage.md) | Daily operation, installation, and local files |
| [Deployment](docs/en/deployment.md) | PostgreSQL, configuration, build, and verification |
| [Private tokens](docs/en/tokens.md) | Product addition and secret rotation through `.env` |
| [Local dashboard](docs/en/local-dashboard.md) | Browser UI, status control, and log deletion |
| [Troubleshooting](docs/en/troubleshooting.md) | Service, connection, and synchronization failures |
| [Security](docs/en/security.md) | Secrets, permissions, and offline limitations |
| [API and storage reference](docs/api.md) | HTTP, SQL, and configuration contract |
| [Arabic documentation](docs/ar/README.md) | Complete Arabic documentation index |
