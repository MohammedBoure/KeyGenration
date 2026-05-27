# Local FastAPI Administration Dashboard

## Role

The FastAPI dashboard is a local administration website only. It is used to:

- display records that reached PostgreSQL table `activation_logs`;
- delete one uploaded record when necessary, after two browser confirmations;
- change `server_control.status` between `1` and `0`.

It is not a backend proxy for the distributed generator and must not be
published as a public service.

## Start the Dashboard

```powershell
cd .\services\local-dashboard
python -m pip install -r .\requirements.txt
Copy-Item .\.env.example .\.env
# Add PostgreSQL settings for an account that can read logs and edit status.
python .\app.py --init-db-only
python .\fastapi_app.py
```

On the same computer, open:

```text
http://127.0.0.1:8080/
```

By default, the site listens locally and rejects requests from other computers.

## Main Page

The page displays:

- current status `0` or `1`;
- buttons to enable or disable generation;
- the newest key records already uploaded to PostgreSQL;
- a selectable visible row limit;
- a `Delete` button for each row.

A record still present in `pending_uploads.json` on a generator computer does
not appear in the dashboard until upload succeeds.

Deletion removes the PostgreSQL record only. If a generator still holds an
unconfirmed queued copy, it can upload that record again later.

## Status Meaning

| Value | Effect |
| --- | --- |
| `1` | Allows installation of a new computer and enables generation after a service receives this value |
| `0` | Rejects new installation and disables new generation after a service receives this value |

Changing status does not stop the Windows service; it disables new key
generation. Previously queued records can still be uploaded. A computer
offline during a change to `0` does not receive the change until reconnection.

## Routes

| Route | Purpose |
| --- | --- |
| `GET /` | Local graphical page |
| `GET /health` | Check dashboard process availability |
| `GET /api/status` | Read current generation status |
| `PUT /api/status` | Write `{"status":"0"}` or `{"status":"1"}` |
| `GET /api/activation-logs?limit=100` | Read newest uploaded records |
| `DELETE /api/activation-logs/{id}` | Delete one record; called by the page after two confirmations |

Example status update on the administration computer:

```powershell
Invoke-RestMethod -Uri http://127.0.0.1:8080/api/status `
  -Method Put -ContentType application/json -Body '{"status":"0"}'
```

## Delete a Record

Use the `Delete` button in the relevant row. The page presents:

1. a first confirmation identifying the selected record;
2. a final confirmation that deletion cannot be undone.

Only after both confirmations does the browser issue the `DELETE` request.
The dashboard PostgreSQL account needs `DELETE` permission on
`activation_logs` for this operation.

## Initialization and Clearing Records

`python .\app.py --init-db-only` initializes schema and does not delete logs.

Before clearing many records, ensure generator computers have no relevant
`pending_uploads.json` items; otherwise older records can reappear after
synchronization. Do not delete `server_control` when clearing log data.

## Protection

- Keep `ADMIN_WEB_HOST=127.0.0.1`.
- Do not set `ADMIN_WEB_ALLOW_REMOTE=1` without authentication, firewall
  controls, and appropriate encrypted transport.
- Use a dedicated dashboard account and protect its `.env`.
- Grant `DELETE` on `activation_logs` only when record deletion is required.
