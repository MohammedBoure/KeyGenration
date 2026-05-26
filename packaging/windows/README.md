# Windows Package

This directory contains Windows distribution inputs only. Application source
lives under `apps/`; generated binaries are written to ignored `dist/`.

```powershell
Copy-Item .\packaging\windows\client.env.example .\packaging\windows\.env
# Edit .\packaging\windows\.env with deployed client API settings.
.\packaging\windows\package.cmd
```

The command starts the PowerShell packager with a script execution policy
suitable for the local packaging invocation, embeds only the permitted values
from `.env` in both Rust executables, and creates:

```text
dist\windows\ActivateurRMS\
|-- ActivateurRMS.exe
|-- SHA256SUMS.txt
|-- KeyGenService\KeyGenService.exe
`-- nssm\nssm.exe
```

`packaging/windows/.env` is a build input and is not copied into the package.
Never place PostgreSQL credentials in it or in the client package. Packaging
fails when `.env` is missing so a production package cannot silently use the
placeholder URL.

For a non-production packaging check only, the template can be selected
explicitly:

```powershell
.\packaging\windows\package.cmd -ClientEnvironmentFile .\packaging\windows\client.env.example
```
