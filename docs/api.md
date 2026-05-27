# API and Storage Reference

This is the technical contract between the Rust UI, the Rust local backend,
PostgreSQL storage, and the local FastAPI dashboard. The current design does
not expose a public cloud HTTP API.

## Rust Local Backend

Default address:

```text
http://127.0.0.1:45632
```

Only `ActivateurRMS.exe` on the generator computer is intended to use this API.

### `GET /health`

Example response:

```json
{
  "status": "ok",
  "backend_mode": "local-queue-v1",
  "token_names": ["Restaurant", "Lab", "Jewelry"],
  "authorized": true,
  "maintenance": false,
  "pending_uploads": 0
}
```

| Field | Meaning |
| --- | --- |
| `backend_mode` | Backend contract version required by the UI |
| `token_names` | Product names loaded from `.env`; the UI compares this list with its configuration |
| `authorized` | Whether `AUTHORIZED.txt` exists after initial authorization |
| `maintenance` | Whether generation is blocked locally after status `0` |
| `pending_uploads` | Number of records still present in the upload queue |

The UI also verifies that the `KeyGenService` Windows service is registered. A
manually launched HTTP listener at the same port is not a supported deployment.

### `POST /generate_key`

Request:

```json
{
  "request_code": "F81A-67A7-C6AA",
  "app_type": "Restaurant"
}
```

Accepted `app_type` values are the `KEYGEN_TOKEN_<ID>_NAME` values configured
in `.env`. If an internal client omits `app_type`, the service uses the first
token listed in `KEYGEN_TOKEN_IDS`.

Successful response:

```json
{
  "request_code": "F81A-67A7-C6AA",
  "activation_key": "EE8C-551F-0A90-73F5",
  "app_type": "Restaurant",
  "status": "generated_and_queued"
}
```

`generated_and_queued` means the key has been generated and stored locally and
a synchronization record has been queued. PostgreSQL upload can occur later.

| HTTP status | Cause |
| --- | --- |
| `400` | Invalid JSON, invalid `XXXX-XXXX-XXXX` request code, or unknown `app_type` |
| `503` | Maintenance mode is active or local persistence failed |

## Administrative Executables

### Desktop UI commands

```powershell
ActivateurRMS.exe --install
ActivateurRMS.exe --uninstall
ActivateurRMS.exe --unstall
ActivateurRMS.exe --help
```

`--unstall` is maintained as an alias for `--uninstall`.

### Internal backend authorization command

```powershell
KeyGenService.exe --authorize-install
```

This command connects to PostgreSQL and succeeds only when
`server_control.status='1'`. The desktop UI invokes it before installing or
updating the Windows service.

## Rust Service Configuration

| Variable | Required | Default | Purpose |
| --- | --- | --- | --- |
| `PGHOST` | Yes | None | PostgreSQL host |
| `PGPORT` | Yes | None | PostgreSQL port |
| `PGDATABASE` | Yes | None | PostgreSQL database |
| `PGUSER` | Yes | None | Restricted generator account |
| `PGPASSWORD` | Yes | None | PostgreSQL password |
| `KEYGEN_TOKEN_IDS` | Yes | None | Ordered comma-separated token identifiers |
| `KEYGEN_TOKEN_<ID>_NAME` | Yes per token | None | Name displayed by the UI and sent as `app_type` |
| `KEYGEN_TOKEN_<ID>_SECRET` | Yes per token | None | Secret used in activation-key derivation |
| `PGSSLMODE` | No | `require` | `disable`, `require`, or `verify-full` |
| `PGCONNECT_TIMEOUT` | No | `10` | Database connection timeout in seconds |
| `KEYGEN_LISTEN_ADDRESS` | No | `127.0.0.1:45632` | Local HTTP listening address |
| `KEYGEN_STATUS_INTERVAL_SECONDS` | No | `15` | Remote status poll interval |
| `KEYGEN_UPLOAD_INTERVAL_SECONDS` | No | `5` | Pending-record upload interval |
| `KEYGEN_DATA_DIR` | No | `%ProgramData%\KeyGenRMS` | Local state folder |
| `KEYGEN_PRIMARY_LOG` | No | `generated_keys.txt` under the data folder | Main generated-key log |

Executables embed only non-sensitive local behavior defaults. PostgreSQL
connection information and all token data, including product names and
secrets, come from private `.env` files during installation and operation.

## Local Storage

| File | Written by | Purpose |
| --- | --- | --- |
| `generated_keys.txt` | Rust service | Text record for each generated key |
| `netcache.dat` | Rust service | Additional local log |
| `pending_uploads.json` | Rust service | Records waiting for PostgreSQL upload |
| `uploaded.log` | Rust service | Trace of completed uploads |
| `AUTHORIZED.txt` | Rust service | Initial authorization marker |
| `MAINTENANCE.txt` | Rust service | Last received disabled state |

Each queued record has a `sync_id` so a repeated synchronization attempt does
not insert the same operation twice.

## PostgreSQL Contract

### Status table

```sql
CREATE TABLE IF NOT EXISTS server_control (
    id INTEGER PRIMARY KEY,
    status TEXT NOT NULL CHECK (status IN ('0', '1'))
);

INSERT INTO server_control (id, status)
VALUES (1, '1')
ON CONFLICT (id) DO NOTHING;
```

The Rust service reads:

```sql
SELECT status FROM server_control WHERE id = 1;
```

### Activation-log table

```sql
CREATE TABLE IF NOT EXISTS activation_logs (
    id BIGSERIAL PRIMARY KEY,
    sync_id TEXT UNIQUE,
    request_code TEXT,
    activation_key TEXT,
    generated_at TEXT,
    device_ip TEXT
);
```

The Rust service synchronizes records with:

```sql
INSERT INTO activation_logs
    (sync_id, request_code, activation_key, generated_at, device_ip)
VALUES ($1, $2, $3, $4, $5)
ON CONFLICT DO NOTHING;
```

`ON CONFLICT DO NOTHING` supports idempotent retries for databases that have
either the original unique constraint or the migration-created partial index
on `sync_id`.

## Local FastAPI Dashboard

Default address:

```text
http://127.0.0.1:8080
```

| Route | Request | Result |
| --- | --- | --- |
| `/` | `GET` | Browser dashboard page |
| `/health` | `GET` | Dashboard process status |
| `/api/status` | `GET` | `{"status":"1"}` or `{"status":"0"}` |
| `/api/status` | `PUT {"status":"0"}` | Updates generator-control status |
| `/api/activation-logs?limit=100` | `GET` | Latest uploaded records |
| `/api/activation-logs/{id}` | `DELETE` | Deletes one uploaded record |

`limit` accepts values from `1` to `500`. By default the dashboard rejects
non-loopback requests unless `ADMIN_WEB_ALLOW_REMOTE=1` is explicitly set;
remote use is not recommended without additional security.

The browser page issues `DELETE` only after two consecutive confirmation
dialogs. The dashboard database account needs `DELETE` permission on
`activation_logs` if record deletion is required.
