# Cloud And Local API Reference

## Cloud API Authentication

All `/api/v1/*` routes require:

```http
Authorization: Bearer <KEYGEN_API_SECRET_TOKEN>
Content-Type: application/json
```

## Cloud Routes

### `GET /api/v1/server-status`

Returns the generation state:

```json
{"status": "1"}
```

`1` is active and `0` is maintenance.

### `POST /api/v1/set-status`

```json
{"status": "0"}
```

### `GET /api/v1/activation-logs`

Returns activation records from PostgreSQL in descending identifier order.

### `POST /api/v1/activation-logs`

Accepts queued service records:

```json
[
  {
    "request_code": "F81A-67A7-C6AA",
    "activation_key": "EE8C-551F-0A90-73F5",
    "generated_at": "2026-05-26T16:00:00.000000+00:00",
    "device_ip": "127.0.0.1"
  }
]
```

## Local Rust Service

The default local address is `http://127.0.0.1:45632`; it can be changed via
`KEYGEN_LISTEN_ADDRESS`.

### `GET /health`

```json
{"status":"ok","maintenance":false,"cloud_api_url":"https://activation.example.com"}
```

### `POST /generate_key`

```json
{
  "request_code": "F81A-67A7-C6AA",
  "app_type": "Restaurant",
  "server_url": "https://activation.example.com"
}
```

Success:

```json
{
  "request_code": "F81A-67A7-C6AA",
  "activation_key": "EE8C-551F-0A90-73F5",
  "app_type": "Restaurant",
  "status": "generated_and_queued"
}
```

During maintenance the route returns HTTP `503` and does not generate a key.

## Local FastAPI Monitoring Dashboard

Run `python .\fastapi_app.py` from `services/cloud-api` to use a local-only
dashboard backed directly by the configured PostgreSQL database.

| Route | Purpose |
| --- | --- |
| `GET /` | Browser dashboard |
| `GET /health` | Web service health response |
| `GET /api/status` | Reads `server_control.status` |
| `PUT /api/status` | Accepts `{"status":"0"}` or `{"status":"1"}` |
| `GET /api/activation-logs?limit=100` | Reads the latest activation records |

By default it accepts browser requests only from the local computer. It is
separate from the bearer-token protected Cloud API used by the Rust service.
