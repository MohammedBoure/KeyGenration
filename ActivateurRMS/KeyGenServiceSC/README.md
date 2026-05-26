# Legacy Python KeyGen Service

This folder contains the Python compatibility backend. New distributions
should build `..\KeyGenServiceRust`, which produces a smaller standalone
`KeyGenService.exe` while preserving the local API contract.

## Local API

```text
POST http://127.0.0.1:45632/generate_key
```

Request body:

```json
{
  "request_code": "F81A-67A7-C6AA",
  "app_type": "Restaurant",
  "server_url": "http://qylad-server.duckdns.org:7002"
}
```

The supported application types are `Restaurant`, `Lab`, and `Jewelry`.

## PostgreSQL Integration

The local service does not connect directly to PostgreSQL. It writes keys to
its offline queue, then submits records to the cloud API:

```text
GET  /api/v1/server-status
POST /api/v1/activation-logs
```

`server.py` persists those requests in PostgreSQL. Configure the legacy
service with:

```powershell
$env:KEYGEN_CLOUD_API_URL = "http://qylad-server.duckdns.org:7002"
$env:KEYGEN_API_SECRET_TOKEN = "<api-token>"
python .\KeyGenService.py
```

No PostgreSQL password should be placed in a desktop backend or executable.
