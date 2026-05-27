# Private Token Configuration

## Purpose

Product choices and key-derivation secrets are not hard-coded into the Rust
programs. On a generator computer, their source is the private `.env` placed
beside `ActivateurRMS.exe`. During service installation the UI writes the
required values to `%ProgramFiles%\KeyGenRMS\.env`.

Never add the real `.env` to git, and never put actual secrets in an example
file or documentation page.

## Format

```dotenv
KEYGEN_TOKEN_IDS=restaurant,lab,jewelry

KEYGEN_TOKEN_RESTAURANT_NAME=Restaurant
KEYGEN_TOKEN_RESTAURANT_SECRET=replace-with-private-restaurant-token

KEYGEN_TOKEN_LAB_NAME=Lab
KEYGEN_TOKEN_LAB_SECRET=replace-with-private-lab-token

KEYGEN_TOKEN_JEWELRY_NAME=Jewelry
KEYGEN_TOKEN_JEWELRY_SECRET=replace-with-private-jewelry-token
```

| Variable | Purpose |
| --- | --- |
| `KEYGEN_TOKEN_IDS` | Ordered list of options displayed in the UI |
| `KEYGEN_TOKEN_<ID>_NAME` | Visible name and `app_type` sent to the backend |
| `KEYGEN_TOKEN_<ID>_SECRET` | Private value used by the Rust backend to derive a key |

`<ID>` supports letters, numbers, and underscores. It is normalized to upper
case for variable lookup: identifier `new_product` uses
`KEYGEN_TOKEN_NEW_PRODUCT_NAME` and `KEYGEN_TOKEN_NEW_PRODUCT_SECRET`.

Each name must be unique, and no secret can be empty.

## Add a Token

For example:

```dotenv
KEYGEN_TOKEN_IDS=restaurant,lab,jewelry,inventory
KEYGEN_TOKEN_INVENTORY_NAME=Inventory
KEYGEN_TOKEN_INVENTORY_SECRET=replace-with-new-private-token
```

After editing `.env` on a managed generator computer, run:

```powershell
.\ActivateurRMS.exe --install
```

This copies the updated list and secrets into the installed service
configuration and restarts it. The UI also compares token names exposed from
`/health`; an old installed list is treated as requiring an update.

## Rotate a Token Secret

Changing a `SECRET` changes future output for the same request code. Therefore:

1. Retain the old secret outside the repository if old-key reproduction is required.
2. Change the value only in approved computers' private `.env` files.
3. Run `ActivateurRMS.exe --install` on each updated generator computer.
4. Perform a test generation and verify that its record uploads.

## Security Boundary

- The Rust service needs tokens locally because it supports offline generation.
- Keeping tokens out of executables and git prevents accidental public
  distribution, but does not defeat a local administrator of a generator PC.
- A token ever committed to git history must be considered exposed and rotated
  before public repository publication.
