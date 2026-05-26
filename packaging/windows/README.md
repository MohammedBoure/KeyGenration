# Windows Package

This directory contains Windows distribution inputs only. Application source
lives under `apps/`; generated binaries are written to ignored `dist/`.

```powershell
.\packaging\windows\package.cmd
```

The command starts the PowerShell packager with a script execution policy
suitable for the local packaging invocation, builds both Rust executables,
and creates:

```text
dist\windows\ActivateurRMS\
|-- ActivateurRMS.exe
|-- .env.example
|-- SHA256SUMS.txt
|-- KeyGenService\KeyGenService.exe
`-- nssm\nssm.exe
```

Rename `.env.example` to `.env` in a distributed copy and fill in the cloud
API settings. Never place PostgreSQL credentials in the client package.
