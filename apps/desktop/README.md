# Activateur RMS Native UI

This crate provides the native Windows Rust desktop program.

The client usage guide is in
[`../../docs/ar/usage.md`](../../docs/ar/usage.md).

## Responsibilities

- Display the activation UI for `Restaurant`, `Lab`, and `Jewelry`.
- Show only application selection, identifier input, activation output, and generation status.
- Refuse to open the generation UI or install the service unless Cloud API status is active.
- Verify hidden cloud API settings and install `KeyGenService.exe` with NSSM automatically.
- Read filtered client settings embedded during the package build.
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
```

Before packaging, copy `packaging/windows/client.env.example` to
`packaging/windows/.env` and supply the cloud API settings. The build embeds
only the filtered client settings in the Rust binaries; it never embeds
PostgreSQL connection values.

When the local service is absent, `ActivateurRMS.exe` requests administrator
permission and installs it automatically. Generation itself does not require
elevation once the service is active. If central status is `0`, startup fails
closed before the generation window or an installation action is available.
