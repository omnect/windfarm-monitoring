# Project Context

This repository implements an IoT Edge module that simulates metrics of a virtual wind farm device and reports them to Azure IoT Hub.

## Rules

- Before committing any changes that affect the project structure, the "Project Structure" section in this file must be updated.

## 1. Architecture & Tech Stack

The project is a single Rust binary that connects to Azure IoT Hub via the `azure-iot-sdk` crate. It reports simulated wind farm telemetry (wind speed, wind direction, location) as D2C messages every 60 seconds and manages its configuration via the IoT Hub device twin.

### Components

- **Twin** (`src/twin.rs`): Orchestrator. Manages the Azure IoT Hub connection, handles desired property updates (location coordinates), reports module/SDK version as twin properties, and drives the main `tokio::select!` event loop.
- **MetricsProvider** (`src/metrics_provider.rs`): Spawns a background task that generates simulated wind metrics every 60 seconds and sends them to the `"metrics"` IoT Edge output queue.
- **Entry point** (`src/main.rs`): Configures logging and calls `Twin::run()`.

### Data Flow

1. Module starts, `Twin` establishes connection to Azure IoT Hub
2. On first authentication, reports module version and Azure SDK version as twin reported properties
3. On desired property update, reads or randomly generates location coordinates (within a German North Sea wind farm region)
4. Reports location as twin reported property and starts `MetricsProvider`
5. Every 60 seconds, `MetricsProvider` generates wind speed and direction (with ±5% random deviation) and sends a JSON message to the `"metrics"` output

### Environment Variables

Set by the IoT Edge runtime:

| Variable | Purpose |
|---|---|
| `IOTEDGE_DEVICEID` | Edge device identifier (used as metric label) |
| `IOTEDGE_IOTHUBHOSTNAME` | IoT Hub hostname (used as metric label) |
| `IOTEDGE_MODULEID` | Module identifier (used as metric label) |

## Build Commands

### Build Docker Image and Deploy to Device

```bash
# Build ARM64 image and deploy to device
./scripts/build-and-deploy-image.sh --arch arm64 --deploy --host <device-ip> --password <ssh-pwd>
```

### Build Rust Binary Locally

```bash
cargo build --release
```

### Run Tests

```bash
cargo test
```

## Project Structure

```text
windfarm-monitoring/
├── Cargo.toml                          # Package manifest with [profile.dist]
├── Cargo.lock
├── Dockerfile                          # Multi-stage Docker build (distroless final image)
├── scripts/
│   └── build-and-deploy-image.sh       # Docker build and deploy script
├── src/
│   ├── main.rs                         # Entry point: logging init, calls Twin::run()
│   ├── twin.rs                         # IoT Hub connection, twin handling, event loop
│   └── metrics_provider.rs             # Background task: generates and sends wind metrics
├── systemd/
│   ├── windfarm-monitoring.service     # systemd service (Type=notify, WatchdogSec=30s)
│   └── windfarm-monitoring.timer       # systemd timer: restarts service if inactive >10min
└── project-context.md                  # This file
```

### Key Files

- `src/twin.rs` — `Twin::run()` is the main event loop; `handle_desired()` starts the metrics provider
- `src/metrics_provider.rs` — `MetricsProvider::run()` spawns the background metrics task; `data_collector()` is the core simulation loop
- `Dockerfile` — Two-stage build: `builder` (compiles with `cargo auditable build --profile dist`) → distroless final image with copied shared libs
- `scripts/build-and-deploy-image.sh` — Builds multi-arch image, optionally pushes to registry or deploys via SSH/iotedge
