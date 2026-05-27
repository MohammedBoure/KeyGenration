# Activateur RMS Native UI

Native Windows Rust interface and NSSM installer.

## Behavior

- Opens the generator window, verifies that the Windows `KeyGenService`
  installed through NSSM is registered, then checks its local health endpoint.
- If the registered service is stopped, attempts to start it without requiring
  a fresh PostgreSQL authorization.
- Accepts only the current `local-queue-v1` backend mode; an obsolete installed
  service is upgraded through the normal installation path.
- When no current service is installed, runs
  `KeyGenService.exe --authorize-install`; installation proceeds only when
  PostgreSQL is reachable and `server_control.status` is `1`.
- Requests administrator permission only after that authorization succeeds.
- Reports the generator as ready only after the installed local service answers
  its health check.
- Once installed, sends generation requests to the local service at
  `http://127.0.0.1:45632/generate_key`.

The UI does not generate keys or write to PostgreSQL itself.

## Build

```powershell
cargo test --workspace
cargo build --release -p activateur-rms
.\packaging\windows\package.cmd -SkipBuild
```

During packaging, only non-secret defaults are embedded. On the computer where
the tool is run, put a configured `.env` beside `ActivateurRMS.exe`. The UI
passes that local configuration to the authorization check and writes it to
the installed service directory; the packaged executables do not embed the
PostgreSQL username or password.
