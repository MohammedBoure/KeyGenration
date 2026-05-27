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

Packaging embeds only non-secret local behavior defaults; it does not embed
PostgreSQL connection details or generation tokens. To operate the packaged
tool on the managed computer, place a configured `.env` beside
`ActivateurRMS.exe`. During authorized installation the UI writes that
configuration to the local NSSM service directory.

The local configuration should use an account restricted to:

- `SELECT` on `server_control`.
- `INSERT` on `activation_logs`.

Do not distribute or expose the local `.env` containing a database password
and the private `KEYGEN_TOKEN_*_SECRET` values.

Generation options are configured as an ordered list:

```dotenv
KEYGEN_TOKEN_IDS=restaurant,new_product
KEYGEN_TOKEN_RESTAURANT_NAME=Restaurant
KEYGEN_TOKEN_RESTAURANT_SECRET=replace-with-private-restaurant-token
KEYGEN_TOKEN_NEW_PRODUCT_NAME=New Product
KEYGEN_TOKEN_NEW_PRODUCT_SECRET=replace-with-private-new-product-token
```

After changing this list on an existing computer, run
`.\ActivateurRMS.exe --install` so the Windows service receives the updated
private configuration.

On the managed computer, the packaged UI can administer its local service:

```powershell
.\ActivateurRMS.exe --install
.\ActivateurRMS.exe --uninstall
```

`--unstall` is accepted as an alias for `--uninstall`.
