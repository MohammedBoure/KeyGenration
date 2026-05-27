# Activateur RMS Native UI

Native Windows Rust interface and NSSM installer.

## Behavior

- Opens the generator window and checks the local `KeyGenService` health endpoint.
- Accepts only the current `local-queue-v1` backend mode; an obsolete installed
  service is upgraded through the normal installation path.
- When no current backend is running, asks for administrator permission and
  runs `KeyGenService.exe --authorize-install`.
- Installation proceeds only when PostgreSQL is reachable and
  `server_control.status` is `1`.
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
