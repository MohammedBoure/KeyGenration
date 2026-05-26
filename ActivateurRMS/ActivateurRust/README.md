# Activateur RMS Native UI

This crate provides the native Windows Rust desktop program.

## Responsibilities

- Display the activation UI for `Restaurant`, `Lab`, and `Jewelry`.
- Verify the cloud API credentials before setup.
- Install or upgrade `KeyGenService.exe` with NSSM.
- Set the service cloud URL and bearer token environment.
- Call the local Rust service to generate activation keys.

Use an HTTPS cloud API address in production so the bearer token and
activation records are encrypted in transit.

## Build

```powershell
cd .\ActivateurRMS
cargo test
cargo build --release -p activateur-rms
Copy-Item .\target\release\ActivateurRMS.exe .\ActivateurRMS.exe
```

Package `ActivateurRMS.exe` alongside these runtime assets:

```text
ActivateurRMS.exe
KeyGenService\KeyGenService.exe
nssm\nssm.exe
```

Run `ActivateurRMS.exe` as administrator when installing or updating the
Windows service. Generation itself does not require elevation once the
service is active.
