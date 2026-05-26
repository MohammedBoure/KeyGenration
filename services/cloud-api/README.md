# Cloud API

Flask service for server-side activation key generation, PostgreSQL-backed
activation logs, and remote maintenance status. Full deployment instructions are in
[`../../docs/ar/deployment.md`](../../docs/ar/deployment.md).

```powershell
python -m pip install -r .\requirements.txt
Copy-Item .\.env.example .\.env
python .\app.py
```

The server `.env` belongs only on the cloud host and must never be distributed
with the Windows client package.

Define two distinct tokens in `.env`:

- `KEYGEN_API_SECRET_TOKEN` is embedded in the client and may request keys only while enabled.
- `KEYGEN_ADMIN_SECRET_TOKEN` controls status and reads logs; never embed or distribute it.

## Local FastAPI Dashboard

The same directory includes a local administration dashboard that reads the
configured PostgreSQL database directly and can set generation status to `1`
or `0`:

```powershell
python .\fastapi_app.py
```

Open `http://127.0.0.1:8080/`. It binds locally and rejects non-local requests
by default. See [`../../docs/ar/local-dashboard.md`](../../docs/ar/local-dashboard.md)
for configuration and security details.
