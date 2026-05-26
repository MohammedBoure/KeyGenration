# KeyGenService Rust Backend

This is the lightweight local backend used by the Flet interface. It exposes
the same endpoint as the legacy Python service:

```text
POST http://127.0.0.1:45632/generate_key
```

The desktop service does not connect directly to PostgreSQL. It queues
generated activation records locally and sends them to the cloud API, whose
storage is PostgreSQL. This prevents distributing database credentials inside
the client executable.

## Build

```powershell
cd .\KeyGenServiceRust
cargo build --release
```

The Windows executable is written to:

```text
target\release\KeyGenService.exe
```

Place that executable in `..\KeyGenService\KeyGenService.exe` for packaging
with the user interface.

## Configuration

Set these environment variables for the Windows service:

```text
KEYGEN_CLOUD_API_URL=http://qylad-server.duckdns.org:7002
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
