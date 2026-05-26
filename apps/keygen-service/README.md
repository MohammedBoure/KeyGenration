# KeyGenService Rust Backend

This is the lightweight local backend used by the native Rust interface. It
exposes the local key-generation endpoint:

The deployment guide is in
[`../../docs/ar/deployment.md`](../../docs/ar/deployment.md).

```text
POST http://127.0.0.1:45632/generate_key
```

The desktop service does not connect directly to PostgreSQL. It queues
generated activation records locally and sends them to the cloud API, whose
storage is PostgreSQL. This prevents distributing database credentials inside
the client executable.

## Build

```powershell
cd <repository-root>
cargo build --release -p keygen-service
```

The Windows executable is written to:

```text
target\release\KeyGenService.exe
```

Run `.\packaging\windows\package.cmd -SkipBuild` to place that executable in
the generated Windows package with the user interface.

## Configuration

During packaging, filtered client settings from `packaging/windows/.env` are
embedded in both Rust executables. The native installer also writes a filtered
internal service configuration so administrative runtime overrides remain
possible:

```text
KEYGEN_CLOUD_API_URL=https://activation.example.com
KEYGEN_API_SECRET_TOKEN=<cloud-api-token>
```

Optional test and runtime settings:

```text
KEYGEN_LISTEN_ADDRESS=127.0.0.1:45632
KEYGEN_DATA_DIR=<override-local-data-folder>
KEYGEN_PRIMARY_LOG=<override-primary-log-file>
KEYGEN_STATUS_INTERVAL_SECONDS=300
KEYGEN_UPLOAD_INTERVAL_SECONDS=5
```
