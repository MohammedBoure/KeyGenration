# Architecture and Data Flow

## Objective

Activateur RMS provides a small Windows UI for generating activation keys for
products configured in a private `.env` file. The system is designed to:

- continue generating keys offline after initial computer authorization;
- save each generated key locally before any network upload;
- synchronize activity to PostgreSQL when connectivity is available;
- let an administrator disable or enable new generation using status `0/1`;
- avoid deploying a public key-generation API.

## Responsibility Boundaries

| Component | Runs on | Responsibilities | Does not do |
| --- | --- | --- | --- |
| `ActivateurRMS.exe` | Generator computer | UI, service install/update/removal, local generation request | Derive keys or write records directly to PostgreSQL |
| `KeyGenService.exe` | Generator computer as a Windows service | Derive keys, local storage, upload queue, status polling | Expose an administration website or edit status |
| `nssm.exe` | Generator computer | Keep the Rust backend running automatically | Application logic |
| PostgreSQL | Remote server | Store status and uploaded generation records | Run the generator UI or derive keys |
| FastAPI dashboard | Administrator computer only | Display/delete logs and edit status | Serve the distributed generator |

## Connections

```text
                         PostgreSQL over SSL
                    +--------------------------+
                    |                          |
                    v                          |
+-------------------------+       +------------------------------+
| Remote PostgreSQL       |<------| Local FastAPI dashboard       |
| server_control          |       | 127.0.0.1:8080 on admin PC   |
| activation_logs         |       +------------------------------+
+-------------------------+
            ^
            | Direct SSL connection for status and upload
            |
+-------------------------+      loopback HTTP only      +------------------+
| KeyGenService.exe/NSSM  |<-----------------------------| ActivateurRMS.exe |
| 127.0.0.1:45632         |                              | Rust UI           |
+-------------------------+                              +------------------+
```

The UI does not contact the dashboard or PostgreSQL while generating a key. It
reads `.env` during installation to authorize the bundled backend and copy the
private service configuration into the installed service directory.

## Installation Flow

1. A private `.env` is placed beside the packaged UI on the generator computer.
2. The operator opens the UI or runs `ActivateurRMS.exe --install`.
3. The UI starts the bundled backend with `--authorize-install`.
4. That authorization connects to PostgreSQL and succeeds only if status is `1`.
5. The UI requests Administrator permission.
6. It copies the backend and NSSM to `%ProgramFiles%\KeyGenRMS`, writes the
   service `.env`, and registers automatic service `KeyGenService`.
7. At first normal startup the service checks status `1` again and creates
   `%ProgramData%\KeyGenRMS\AUTHORIZED.txt`.
8. The UI accepts readiness only when `GET /health` succeeds and its
   `token_names` match the local `.env` configuration.

If the service executable is missing or an older service exposes a different
backend/token contract, the same authorized installation path repairs it.

## Generation Flow

```text
Operator selects configured product and enters request code
              |
              v
ActivateurRMS.exe -> POST /generate_key -> KeyGenService.exe
                                          |
                                          | SHA-256 using private token in .env
                                          v
                            Key + local log + pending_uploads.json
                                          |
                                  when network is available
                                          v
                                  INSERT activation_logs
```

The request code format is `XXXX-XXXX-XXXX`. The selected product is one of
the names defined through `KEYGEN_TOKEN_IDS` and its corresponding
`KEYGEN_TOKEN_<ID>_NAME`; no product list or private token is compiled into
the executable.

## Queue and Synchronization

- Every generated key is written locally first.
- A JSON record with a unique `sync_id` is appended to `pending_uploads.json`.
- The service attempts upload periodically, every 5 seconds by default.
- PostgreSQL insertion uses `ON CONFLICT DO NOTHING` for safe retries.
- Successfully committed records are removed from the local queue and noted in
  `uploaded.log`.

## Status Control

The service reads `server_control.status` periodically, every 15 seconds by
default:

| Value | Behavior |
| --- | --- |
| `1` | Permit new generation and remove `MAINTENANCE.txt` if present |
| `0` | Create `MAINTENANCE.txt` and reject new generation |

An offline generator continues according to its last received state. A status
change cannot affect it until it reconnects.

## Two Local Services, Different Roles

| Process | Used by | Default address | Required permanently |
| --- | --- | --- | --- |
| Rust backend service | Generator UI | `127.0.0.1:45632` | Yes, as NSSM service on a generator PC |
| FastAPI dashboard | Administrator in a browser | `127.0.0.1:8080` | No, only when administration is needed |
