# Windows Package

This directory creates the Windows UI and permanent NSSM service bundle.

```powershell
Copy-Item .\packaging\windows\client.env.example .\packaging\windows\.env
.\packaging\windows\package.cmd
```

Output:

```text
dist\windows\ActivateurRMS\
|-- ActivateurRMS.exe
|-- SHA256SUMS.txt
|-- KeyGenService\KeyGenService.exe
`-- nssm\nssm.exe
```

Packaging embeds only non-secret defaults; it does not embed `PGUSER` or
`PGPASSWORD`. To operate the packaged tool on the managed computer, place a
configured `.env` beside `ActivateurRMS.exe`. During authorized installation
the UI writes that configuration to the local NSSM service directory.

The local configuration should use an account restricted to:

- `SELECT` on `server_control`.
- `INSERT` on `activation_logs`.

Do not distribute or expose the local `.env` containing a database password.
