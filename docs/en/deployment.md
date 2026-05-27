# Build, Deployment, and Administration

## Deployment Topology

Do not deploy a cloud API for the generator. The required deployment is:

| Location | Installed software |
| --- | --- |
| Remote server | PostgreSQL only |
| Generator computer | Rust package and local NSSM service |
| Administrator computer | Local FastAPI dashboard when needed |

The Rust service and the FastAPI dashboard have independent PostgreSQL
connections. The dashboard does not need to be running for key generation.

## Requirements

- Windows for building and operating the `.exe` bundle and NSSM service.
- A Rust toolchain for building the UI and service.
- Python 3 and the requirements in `services\cloud-api\requirements.txt` for
  database initialization and the dashboard.
- PostgreSQL reachable over SSL from authorized operating computers.

## PostgreSQL Setup

From the repository root:

```powershell
cd .\services\cloud-api
python -m pip install -r .\requirements.txt
Copy-Item .\.env.example .\.env
# Edit .env with an account allowed to initialize the tables.
python .\app.py --init-db-only
```

This creates or updates:

```text
server_control     Single control row: id=1, status='0' or '1'
activation_logs    Uploaded key-generation records
```

If no control row exists, initialization starts with
`server_control.status='1'`.

### Separate Accounts

The dashboard account needs to read logs, update status, and optionally delete
records. The private `.env` placed on a generator computer should use a much
more restricted account, for example:

```sql
GRANT CONNECT ON DATABASE your_database TO restricted_client_user;
GRANT USAGE ON SCHEMA public TO restricted_client_user;
GRANT SELECT ON TABLE server_control TO restricted_client_user;
GRANT INSERT ON TABLE activation_logs TO restricted_client_user;
GRANT USAGE, SELECT ON SEQUENCE activation_logs_id_seq TO restricted_client_user;
```

Do not grant a generator account `UPDATE` on `server_control` or `DELETE` on
records.

## Generator `.env`

Create `packaging\windows\.env` for a private local build and use a similarly
configured `.env` beside the package on each managed generator computer:

```dotenv
PGHOST=database-host
PGPORT=database-port
PGDATABASE=database-name
PGUSER=restricted_client_user
PGPASSWORD=restricted-client-password
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

For the token schema and extension procedure, see
[private token configuration](tokens.md).

Optional advanced settings:

| Variable | Purpose |
| --- | --- |
| `KEYGEN_DATA_DIR` | Override local state directory |
| `KEYGEN_PRIMARY_LOG` | Override `generated_keys.txt` path |

## Dashboard `.env`

Create `services\cloud-api\.env` locally on the administrator computer:

```dotenv
PGHOST=database-host
PGPORT=database-port
PGDATABASE=database-name
PGUSER=dashboard-administrator-user
PGPASSWORD=dashboard-password
PGSSLMODE=require
PGCONNECT_TIMEOUT=10
ADMIN_WEB_HOST=127.0.0.1
ADMIN_WEB_PORT=8080
```

The dashboard account needs `DELETE` on `activation_logs` only if the browser
record-deletion feature will be used.

## Build the Windows Package

From the repository root:

```powershell
Copy-Item .\packaging\windows\client.env.example .\packaging\windows\.env
# Edit the private .env locally; do not commit it.
cargo fmt --all
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
.\packaging\windows\package.cmd
```

Output:

```text
dist\windows\ActivateurRMS\
|-- ActivateurRMS.exe
|-- SHA256SUMS.txt
|-- KeyGenService\KeyGenService.exe
`-- nssm\nssm.exe
```

Packaging embeds only non-sensitive local behavior defaults. It does not embed
PostgreSQL connection details, token names, or token secrets, and it removes
old `.env` files from the output folder. Place the real `.env` beside the UI
only after delivering the package to a managed computer.

## Install or Update a Generator Computer

```powershell
cd .\ActivateurRMS
.\ActivateurRMS.exe --install
```

Behavior:

1. The UI reads local `.env`, including tokens.
2. It asks the bundled backend to authorize installation against PostgreSQL.
3. Installation continues only while remote status is `1`.
4. Windows requests Administrator/UAC permission.
5. Any previous service registration is stopped and replaced.
6. Files are copied to `%ProgramFiles%\KeyGenRMS`.
7. The service is registered for automatic startup and started.
8. The UI verifies that the local backend responds with the expected token list.

Run the same command after upgrading the package or changing a token name or
secret. Copying new binaries alone does not update the installed service.

## Remove the Service

```powershell
.\ActivateurRMS.exe --uninstall
```

or:

```powershell
.\ActivateurRMS.exe --unstall
```

Removal deletes the Windows service and `%ProgramFiles%\KeyGenRMS`, but not
`%ProgramData%\KeyGenRMS`. Inspect `pending_uploads.json` before manually
removing data, since it may contain records not uploaded yet.

## Run the Local Dashboard

```powershell
cd .\services\cloud-api
python .\fastapi_app.py
```

Open `http://127.0.0.1:8080/`. See the
[local dashboard guide](local-dashboard.md) for controls and routes.

## Deployment Acceptance Test

1. Set status to `1` from the dashboard.
2. Run `ActivateurRMS.exe --install` on a test generator computer.
3. Confirm Windows service `KeyGenService` is running.
4. Confirm `%ProgramData%\KeyGenRMS\AUTHORIZED.txt` exists.
5. Generate a key and verify it appears on the dashboard.
6. Disconnect the network, generate a key, and confirm it is queued locally.
7. Restore connectivity and confirm the queued record appears in PostgreSQL.
8. While connected, set status to `0` and confirm new generation is rejected.
9. Disconnect again and confirm the received disabled state remains enforced.
10. Set status back to `1`, reconnect, and confirm generation resumes.
