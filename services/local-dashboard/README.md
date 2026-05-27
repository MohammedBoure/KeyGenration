# Local PostgreSQL Administration Dashboard

This local-only service provides:

- PostgreSQL table initialization and optional SQLite import through `app.py`.
- A local FastAPI dashboard through `fastapi_app.py`.

`KeyGenService.exe` connects directly to PostgreSQL to read status and upload
locally queued activation records.

## Setup

```powershell
python -m pip install -r .\requirements.txt
Copy-Item .\.env.example .\.env
# Edit .env with the administrator PostgreSQL connection.
python .\app.py --init-db-only
python .\fastapi_app.py
```

Open `http://127.0.0.1:8080/`.

The dashboard reads `activation_logs`, deletes an individual record after two
browser confirmations, and updates `server_control.status`. Its PostgreSQL
account therefore needs `DELETE` on `activation_logs` when deletion is used.
It binds locally and rejects non-local requests by default.

## Source Layout

| Path | Responsibility |
| --- | --- |
| `app.py` | PostgreSQL initialization and optional SQLite import command |
| `fastapi_app.py` | Local dashboard server entry point |
| `dashboard_app/` | Configuration, queries, routes, and dashboard template |
| `tests/` | Dashboard and database configuration tests |
