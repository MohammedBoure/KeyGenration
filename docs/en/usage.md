# User Guide

This guide is for the operator using a Windows generator computer. For
database and packaging work, see the [deployment guide](deployment.md).

## What the UI Provides

`ActivateurRMS.exe` displays:

- a product selector populated from the local private `.env`;
- a request-code input in `XXXX-XXXX-XXXX` format;
- a button that generates and displays an activation key.

The UI never derives keys itself. It sends generation requests to the installed
Rust service named `KeyGenService`; the service generates, stores, and later
uploads the record.

## Package Contents

```text
ActivateurRMS\
|-- ActivateurRMS.exe
|-- .env                         # supplied locally, never distributed publicly
|-- SHA256SUMS.txt
|-- KeyGenService\KeyGenService.exe
`-- nssm\nssm.exe
```

Keep the `KeyGenService\` and `nssm\` folders beside the UI, especially when
installing, updating, repairing, or removing the service.

## First Installation

Before starting, verify:

1. The computer runs Windows and can reach the configured PostgreSQL server.
2. A correct private `.env` is beside `ActivateurRMS.exe`; it includes
   database credentials, token names, and token secrets.
3. The administration dashboard reports status `1`.
4. You can approve the Windows Administrator/UAC prompt.

### Through the UI

1. Start `ActivateurRMS.exe`.
2. The UI checks the Windows service and its local backend response.
3. If service installation, update, or repair is needed, the UI first checks
   PostgreSQL authorization.
4. With status `1`, it requests Administrator permission and installs the service.
5. The Rust backend writes `AUTHORIZED.txt` after its first authorization.
6. Once local health checks pass, the UI is ready for generation.

### Through PowerShell or Command Prompt

To force installation, update, or repair:

```powershell
cd .\dist\windows\ActivateurRMS
.\ActivateurRMS.exe --install
```

This needs PostgreSQL connectivity, status `1`, and UAC approval.

## Daily Generation

1. Start `ActivateurRMS.exe`.
2. Select one of the product names configured in `.env`.
3. Enter the request code, for example `F81A-67A7-C6AA`.
4. Select the generation button.
5. Provide the displayed activation key to its intended recipient.

The displayed result confirms local generation and queuing. It does not by
itself prove immediate PostgreSQL upload; offline operations are uploaded later.

## Offline Behavior

| Situation | Behavior |
| --- | --- |
| Computer was authorized, last received status is `1`, network unavailable | Generation works and records queue locally |
| Computer received status `0` before losing network | New generation remains blocked |
| New installation with no network | Installation cannot complete |
| Administrator sets `0` while generator is offline | The computer receives the disable command only after reconnecting |

## Backend Service Commands

```powershell
.\ActivateurRMS.exe --install
.\ActivateurRMS.exe --uninstall
.\ActivateurRMS.exe --unstall
.\ActivateurRMS.exe --help
```

| Option | Purpose |
| --- | --- |
| `--install` | Install, update, or repair `KeyGenService` |
| `--uninstall` | Remove service registration and installed program files |
| `--unstall` | Supported alias for removal |
| `--help` | Display the accepted command options |

Removal deletes `%ProgramFiles%\KeyGenRMS` but preserves
`%ProgramData%\KeyGenRMS`, including generated records and any pending queue.

## Local Paths

Installed application files:

```text
%ProgramFiles%\KeyGenRMS\
|-- KeyGenService.exe
|-- nssm.exe
`-- .env
```

Persistent operational data:

```text
%ProgramData%\KeyGenRMS\
|-- generated_keys.txt
|-- netcache.dat
|-- pending_uploads.json
|-- uploaded.log
|-- AUTHORIZED.txt
`-- MAINTENANCE.txt
```

| File | Meaning |
| --- | --- |
| `.env` under Program Files | Installed private database and token configuration |
| `pending_uploads.json` | Items not yet cleared after upload confirmation |
| `AUTHORIZED.txt` | This computer passed first authorization |
| `MAINTENANCE.txt` | This computer received disable status `0` |

## Updating a Computer

1. Place the new package together with the correct private `.env`.
2. Run:

```powershell
.\ActivateurRMS.exe --install
```

3. Approve UAC.
4. Open the UI and perform a test generation if permitted.
5. Check the dashboard for uploaded records.

Merely launching a new UI is not always enough to replace the installed
service; `--install` is the explicit update path.

Use the same process after adding a token or changing a token name or secret.
The UI compares token names with the installed backend health response and
enters the authorized update path when the installed list is old.

## Related Guides

- [Private token configuration](tokens.md)
- [Troubleshooting](troubleshooting.md)
- [Security and control limits](security.md)
