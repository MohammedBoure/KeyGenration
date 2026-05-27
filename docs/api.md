# Interfaces And Storage Reference

The Windows client no longer uses an HTTP Cloud API. `KeyGenService.exe`
connects directly to PostgreSQL for status reads and deferred log uploads.
The only web application is the local FastAPI dashboard.

## Local Rust Service

Default address: `http://127.0.0.1:45632`.

### `GET /health`

```json
{
  "status": "ok",
  "backend_mode": "local-queue-v1",
  "authorized": true,
  "maintenance": false,
  "pending_uploads": 0
}
```

The desktop UI first requires a registered Windows `KeyGenService` service,
starts it if it is stopped, and requires `backend_mode` to match. Otherwise it
installs or upgrades the service after an authorized online check. A fresh
service does not listen until PostgreSQL returns status `1` and its local
authorization marker has been created.

### `POST /generate_key`

Request:

```json
{"request_code":"F81A-67A7-C6AA","app_type":"Restaurant"}
```

Success:

```json
{
  "request_code":"F81A-67A7-C6AA",
  "activation_key":"EE8C-551F-0A90-73F5",
  "app_type":"Restaurant",
  "status":"generated_and_queued"
}
```

The service refreshes status from PostgreSQL in the background. Requests use
the last locally saved state, allowing offline generation after a successful
initial authorization when it was last active.

## PostgreSQL Contract

### `server_control`

```sql
SELECT status FROM server_control WHERE id = 1;
```

- `1`: permits installation and clears local maintenance when received.
- `0`: refuses a new installation and stops generation once a running service
  receives it.

### `activation_logs`

The local service queues generated records on disk, then inserts them in a
transaction when PostgreSQL becomes reachable:

```sql
INSERT INTO activation_logs
    (sync_id, request_code, activation_key, generated_at, device_ip)
VALUES ($1, $2, $3, $4, $5)
ON CONFLICT DO NOTHING;
```

`sync_id` makes retrying an interrupted upload idempotent.

The local runtime `.env` database account should only have `SELECT` permission
on `server_control` and `INSERT` permission on `activation_logs`. The packaged
executables do not embed `PGUSER` or `PGPASSWORD`.

## Local FastAPI Dashboard

Run from `services/cloud-api`:

```powershell
python .\fastapi_app.py
```

By default it listens on `http://127.0.0.1:8080/`.

| Route | Purpose |
| --- | --- |
| `GET /` | Local browser dashboard |
| `GET /health` | Web service health |
| `GET /api/status` | Read `server_control.status` |
| `PUT /api/status` | Write `{"status":"0"}` or `{"status":"1"}` |
| `GET /api/activation-logs?limit=100` | Display recently uploaded records |
