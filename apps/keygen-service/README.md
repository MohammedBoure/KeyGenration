# KeyGenService Rust Backend

Permanent local backend installed with NSSM by the desktop application.

## Behavior

- `--authorize-install` connects directly to PostgreSQL and succeeds only when
  `server_control.status` is `1`.
- A fresh normal startup also requires status `1`, then writes
  `AUTHORIZED.txt`; only an already authorized installation can start offline.
- On later startup, restores the last maintenance state saved on disk and
  refreshes it from PostgreSQL whenever connectivity is available.
- Generates activation keys locally, including while offline if the last known
  state permits generation.
- Saves generated keys locally and appends records to `pending_uploads.json`.
- Repeatedly attempts to insert pending records into PostgreSQL, clearing the
  queue only after a committed upload.

Default local storage on Windows is `%ProgramData%\KeyGenRMS`, including
`AUTHORIZED.txt`, `MAINTENANCE.txt`, and the pending/upload logs.

## Local API

```text
GET  http://127.0.0.1:45632/health
POST http://127.0.0.1:45632/generate_key
```

## Configuration

On the managed computer, a `.env` beside the desktop executable supplies the
database account during first installation. The installed service then
receives an internal generated `.env` containing:

```dotenv
PGHOST=database-host
PGPORT=9005
PGDATABASE=keygen_restaurant
PGUSER=restricted_client_user
PGPASSWORD=restricted_client_password
PGSSLMODE=require
PGCONNECT_TIMEOUT=10
KEYGEN_LISTEN_ADDRESS=127.0.0.1:45632
KEYGEN_STATUS_INTERVAL_SECONDS=15
KEYGEN_UPLOAD_INTERVAL_SECONDS=5
```

Use a restricted database account in any distributed build: it must not own
the database or be allowed to change `server_control`.
