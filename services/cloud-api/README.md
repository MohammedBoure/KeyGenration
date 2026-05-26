# Cloud API

Flask service for PostgreSQL-backed activation logs and remote maintenance
status. Full deployment instructions are in
[`../../docs/ar/deployment.md`](../../docs/ar/deployment.md).

```powershell
python -m pip install -r .\requirements.txt
Copy-Item .\.env.example .\.env
python .\app.py
```

The server `.env` belongs only on the cloud host and must never be distributed
with the Windows client package.
