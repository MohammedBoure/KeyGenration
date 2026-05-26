# Cloud And Local API Reference

## Cloud API Authentication

The distributed Rust client contains only the limited client token:

```http
Authorization: Bearer <KEYGEN_API_SECRET_TOKEN>
Content-Type: application/json
```

It may call `GET /api/v1/server-status` and `POST /api/v1/generate-key`.
Administration routes require the separate token that must never be included
in a client build:

```http
Authorization: Bearer <KEYGEN_ADMIN_SECRET_TOKEN>
```

## Cloud Routes

### `GET /api/v1/server-status`

Returns the generation state:

```json
{"status": "1"}
```

`1` is active and `0` is maintenance.

### `POST /api/v1/generate-key`

The key algorithm exists only in Cloud API. Generation succeeds only while
`server_control.status` is `1`, and the record is inserted in PostgreSQL in
the same request.

```json
{"request_code":"F81A-67A7-C6AA","app_type":"Restaurant"}
```

Success:

```json
{"request_code":"F81A-67A7-C6AA","activation_key":"EE8C-551F-0A90-73F5","app_type":"Restaurant","status":"generated_and_recorded"}
```

When disabled it returns HTTP `503` without generating a key.

### `POST /api/v1/set-status` (admin only)

```json
{"status": "0"}
```

### `GET /api/v1/activation-logs` (admin only)

Returns activation records from PostgreSQL in descending identifier order.

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
  "app_type": "Restaurant"
}
```

Success:

```json
{
  "request_code": "F81A-67A7-C6AA",
  "activation_key": "EE8C-551F-0A90-73F5",
  "app_type": "Restaurant",
  "status": "generated_by_cloud"
}
```

The local backend is a proxy and does not contain the key algorithm. It asks
Cloud API for every key. During maintenance or when remote authorization
cannot be verified, it returns HTTP `503` and does not generate a key.

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
