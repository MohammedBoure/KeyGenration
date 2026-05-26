# Activateur RMS Native UI

This crate provides the native Windows Rust desktop program.

The client usage guide is in
[`../../docs/ar/usage.md`](../../docs/ar/usage.md).

## Responsibilities

- Display the activation UI for `Restaurant`, `Lab`, and `Jewelry`.
- Verify the cloud API credentials before setup.
- Install or upgrade `KeyGenService.exe` with NSSM.
- Read package settings from `.env` and write filtered service settings.
- Call the local Rust service to generate activation keys.

Use an HTTPS cloud API address in production so the bearer token and
activation records are encrypted in transit.

## Build

```powershell
cd <repository-root>
cargo test
cargo build --release -p activateur-rms
.\packaging\windows\package.cmd -SkipBuild
```

The package script writes these runtime assets under
`dist\windows\ActivateurRMS`:

```text
ActivateurRMS.exe
KeyGenService\KeyGenService.exe
nssm\nssm.exe
.env.example
```

Rename the packaged `.env.example` to `.env` and supply the cloud API
settings. The installed service receives only client runtime settings and
never PostgreSQL connection values.

Run `ActivateurRMS.exe` as administrator when installing or updating the
Windows service. Generation itself does not require elevation once the
service is active.
