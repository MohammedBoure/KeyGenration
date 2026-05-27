# Troubleshooting and Maintenance

## Basic Checks

On a generator computer:

```powershell
sc.exe query KeyGenService
sc.exe qc KeyGenService
Invoke-RestMethod http://127.0.0.1:45632/health
Get-Content "$env:ProgramData\KeyGenRMS\pending_uploads.json"
```

On the administration computer while the dashboard is running:

```powershell
Invoke-RestMethod http://127.0.0.1:8080/api/status
Invoke-RestMethod "http://127.0.0.1:8080/api/activation-logs?limit=100"
```

## `Demarrage du service impossible`

Possible causes:

- the service points to an older or missing executable;
- files under `%ProgramFiles%\KeyGenRMS` are incomplete;
- the previous installation cannot start;
- Administrator permission was not approved.

Repair from a current package folder containing its private `.env`:

```powershell
.\ActivateurRMS.exe --install
```

Approve UAC. The command authorizes the installation against PostgreSQL and
then replaces the installed service files and registration.

## Installation Fails

| Symptom | Likely cause | Action |
| --- | --- | --- |
| Installation refused as disabled | `server_control.status=0` | Set status to `1` from the dashboard |
| PostgreSQL connection error | No network or incorrect `.env` | Review host, port, SSL mode, account, and password |
| UAC was rejected | Administrator approval not granted | Repeat `--install` and approve UAC |
| Backend or NSSM missing | Incomplete package | Copy the full package folders together |
| `Configuration des tokens invalide` | Missing or duplicate token setting | Check `KEYGEN_TOKEN_IDS`, `NAME`, and `SECRET` for each identifier |

## UI Opens but Generation Fails

Check local backend health:

```powershell
Invoke-RestMethod http://127.0.0.1:45632/health
```

Important response fields:

| Field/result | Meaning |
| --- | --- |
| `backend_mode` is not `local-queue-v1` | Installed backend needs update |
| `token_names` differs from current `.env` | Run `ActivateurRMS.exe --install` to update service configuration |
| `authorized` is `false` | Initial authorization has not completed |
| `maintenance` is `true` | Status `0` has disabled new generation |

## `pending_uploads` Does Not Reach Zero

If health reports pending items, verify:

1. network access from the generator computer to PostgreSQL;
2. generator account permissions for inserting into `activation_logs`;
3. that the installed service is updated by running:

```powershell
.\ActivateurRMS.exe --install
```

Do not simply delete `pending_uploads.json`; it can contain records that have
not reached PostgreSQL.

## Records Reappear After Database Clearing

A generator may still have queued records locally and upload them after you
cleared PostgreSQL. Before clearing records:

1. inspect pending queues on each generator computer;
2. allow synchronization to complete;
3. ensure pending counts are zero;
4. clear only `activation_logs`, retaining `server_control`.

## Dashboard Does Not Start or Display Data

The dashboard now requires every private PostgreSQL setting in its local `.env`:

```powershell
cd .\services\cloud-api
Copy-Item .\.env.example .\.env
```

Edit `PGHOST`, `PGPORT`, `PGDATABASE`, `PGUSER`, and `PGPASSWORD`, then start
the dashboard again.

Requests from another computer are rejected by default because the dashboard
is deliberately local-only.

## Update or Removal

To apply a new backend or token configuration:

```powershell
.\ActivateurRMS.exe --install
```

To remove the backend service while retaining operational data:

```powershell
.\ActivateurRMS.exe --uninstall
```
