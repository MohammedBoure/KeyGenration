# Security and Control Limits

## Threat Model

The system provides operational control over generator use and a history of
uploaded generation events. It does not promise absolute protection from a
person who controls a generator computer and has sufficient time to analyze it.

Three constraints matter:

1. Offline generation requires local key-derivation logic and private tokens.
2. Direct PostgreSQL synchronization requires local connection credentials.
3. A remote disable instruction cannot reach a disconnected computer.

## Secrets and `.env` Files

| File | Typically contains | Where it must remain |
| --- | --- | --- |
| `.env` beside `ActivateurRMS.exe` | Generator database account and `KEYGEN_TOKEN_*_SECRET` values | Managed generator computer only |
| `%ProgramFiles%\KeyGenRMS\.env` | Installed service copy, including tokens | Managed generator computer only |
| `services\cloud-api\.env` | Dashboard database account | Administrator computer only |

Mandatory rules:

- Never commit `.env` or another real `*.env` file.
- Never deliver a package containing `.env` to an untrusted party.
- Never put actual token values in source code, tests, documentation, or example files.
- Never use the database-owner account on a seller's generator computer.
- Rotate credentials and tokens after loss of trust in a machine or package.

Packaging removes `.env` from its output and does not embed PostgreSQL details
or `KEYGEN_TOKEN_*_SECRET` values in executable files.

If older generation secrets existed in a previous git commit, deleting them in
the latest code does not remove them from history. Rotate them before a public
release and consider history cleanup before publication.

## Least-Privilege PostgreSQL Account

The generator service needs:

- database connection;
- read access to `server_control`;
- insert access to `activation_logs`;
- sequence access needed for record IDs.

Example:

```sql
GRANT CONNECT ON DATABASE your_database TO restricted_client_user;
GRANT USAGE ON SCHEMA public TO restricted_client_user;
GRANT SELECT ON TABLE server_control TO restricted_client_user;
GRANT INSERT ON TABLE activation_logs TO restricted_client_user;
GRANT USAGE, SELECT ON SEQUENCE activation_logs_id_seq TO restricted_client_user;
```

Do not grant the generator account status updates, record deletion, role
management, or database ownership.

The dashboard needs a different administrative account able to read records
and edit status. Give it `DELETE` on `activation_logs` only if the delete
button is intended to be usable.

## Connection Encryption

External database connections require SSL. Supported service modes are:

| `PGSSLMODE` | Effect |
| --- | --- |
| `require` | Encrypts traffic but does not require certificate identity verification |
| `verify-full` | Encrypts traffic and verifies server identity when certificates are correctly configured |
| `disable` | Unencrypted; unsuitable for remote use |

Prefer `verify-full` when the server deployment provides a trusted certificate.

## Status `0` Limit

After the service receives status `0`, it writes `MAINTENANCE.txt` and rejects
new generation, including while offline. However, a device disconnected before
receiving `0` continues based on its previous allowed state until reconnection.

This limitation follows from the requirement to permit offline generation.

## Initial Authorization and Removal

A fresh service must connect to PostgreSQL and observe status `1` before
creating `AUTHORIZED.txt`. This prevents first installation while disabled.

`ActivateurRMS.exe --uninstall` removes the Windows service and installed
program files but preserves `%ProgramData%\KeyGenRMS`. Verify pending upload
records before manually erasing that persistent folder.

## Dashboard Security

The FastAPI site is local-only by default. Treat the administration computer as
sensitive because it can change generator status and, when permitted, delete
uploaded logs. If remote access is ever enabled, add authentication, network
restrictions, encryption, and audit controls first.
