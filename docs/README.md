# Activateur RMS Documentation

English is the primary documentation language for the current version.
Complete Arabic documentation is also maintained in [ar/README.md](ar/README.md).

The deployed design has no public cloud API for the Windows generator. A
permanent local Rust service runs on each generator computer, a separate
FastAPI dashboard is used locally by an administrator, and both communicate
with PostgreSQL directly.

## Start Here

| Need | Guide |
| --- | --- |
| Understand components and data flow | [Architecture and data flow](en/architecture.md) |
| Operate a generator computer | [User guide](en/usage.md) |
| Build, configure, and deploy | [Deployment guide](en/deployment.md) |
| Add products and manage private tokens | [Private token configuration](en/tokens.md) |
| Run the website and control status/logs | [Local FastAPI dashboard](en/local-dashboard.md) |
| Diagnose failures | [Troubleshooting](en/troubleshooting.md) |
| Understand secret handling and limits | [Security and control limits](en/security.md) |
| Integrate with HTTP or PostgreSQL | [API and storage reference](api.md) |

## Quick Map

```text
ActivateurRMS.exe  -->  KeyGenService.exe/NSSM  -->  PostgreSQL
                               ^
                               | loopback HTTP only

Local FastAPI dashboard on admin computer --------> PostgreSQL
```

## Terminology

| Term | Meaning |
| --- | --- |
| UI | `ActivateurRMS.exe`, used by the person generating keys |
| Local backend | `KeyGenService.exe`, installed as a Windows service through NSSM |
| Dashboard | Local FastAPI website for monitoring and control |
| Remote server | PostgreSQL storage only, not a key-generation API |
| Status | `server_control.status`, either `1` to enable or `0` to disable new generation |
| Token | Private per-product secret read from `.env` and used for local key derivation |

## Source Layout

| Path | Purpose |
| --- | --- |
| `apps\desktop` | Rust desktop UI and service administration commands |
| `apps\keygen-service` | Rust local backend service |
| `crates\config` | Shared `.env` and token configuration parsing |
| `services\cloud-api` | Historical folder name for the local FastAPI dashboard and database initializer |
| `packaging\windows` | Windows bundle and NSSM packaging scripts |
| `docs\en` | Detailed English operational guides |
| `docs\ar` | Detailed Arabic operational guides |
| `dist\windows\ActivateurRMS` | Local build output, ignored by git |
