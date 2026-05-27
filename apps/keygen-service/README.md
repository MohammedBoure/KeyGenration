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
- Reads the selectable product names and their private generation tokens from
  its installed `.env`; no product token is compiled into the executable.
- Saves generated keys locally and appends records to `pending_uploads.json`.
- Repeatedly attempts to insert pending records into PostgreSQL, clearing the
  queue only after a committed upload.

Default local storage on Windows is `%ProgramData%\KeyGenRMS`, including
`AUTHORIZED.txt`, `MAINTENANCE.txt`, and the pending/upload logs.

## Source Layout

| Module | Responsibility |
| --- | --- |
| `src/main.rs` | Startup, authorization state, polling, and upload orchestration |
| `src/database.rs` | PostgreSQL settings, TLS connection, and remote status access |
| `src/http.rs` | Loopback HTTP API and JSON responses |
| `src/storage.rs` | Persistent local logs and pending upload queue |
| `src/generation.rs` | Request-code checks and deterministic key generation |

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
PGPORT=replace-with-postgresql-port
PGDATABASE=keygen_restaurant
PGUSER=restricted_client_user
PGPASSWORD=restricted_client_password
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

Use a restricted database account in any distributed build: it must not own
the database or be allowed to change `server_control`. To add a product, append
an identifier to `KEYGEN_TOKEN_IDS`, add its `NAME` and `SECRET` variables,
then reinstall/update the service through `ActivateurRMS.exe --install`.
