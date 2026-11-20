# windfarm-monitoring

Azure IoT Edge module for demos. It simulates a wind farm device and sends
latitude, longitude, wind speed and wind direction as D2C messages to the
output `metrics` once per minute.

## Structure

- `src/main.rs`: logging setup, then `Twin::run()`.
- `src/twin.rs`: event loop; creates the edge client, reports module and
  SDK version, reads the location from the desired properties and starts
  the metrics task. Without a location in the desired properties it picks a
  random one in a fixed North Sea area.
- `src/metrics_provider.rs`: the metrics task. It draws a random base value
  per hour of the day and adds up to ±5% noise on each sample.
- `Dockerfile`: builds with `cargo auditable build --profile dist` and
  copies the binary and the shared libraries it needs into a distroless
  image.
- `scripts/build-and-deploy-image.sh`: local developer script; builds the
  image, can push it (`--push`) or load it on a device over SSH (`--deploy
  --host <ip>`). See `--help` for all options.
- `systemd/`: old unit files. The module runs as a container, nothing
  installs them, and the code does not send `sd_notify`.

## Constraints

- The module only runs under the IoT Edge runtime. It uses the edge client,
  and it reads `IOTEDGE_DEVICEID`, `IOTEDGE_IOTHUBHOSTNAME` and
  `IOTEDGE_MODULEID` as metric labels. If one is missing, the metrics task
  panics on the first sample; the `dist` profile has `panic = "abort"`,
  so the image stops the whole module.
- The location is set once per start. Later desired property updates do not
  change it.
- The IoT Edge deployment needs a route from the `metrics` output to
  `$upstream`.
- `README.md` documents the message format. Change it together with the
  format.

## Commands

- Build: `cargo build`. It needs the Azure IoT C SDK: pkg-config
  `azure-iotedge-sdk-dev`, or `AZURESDK_PATH`, `UUID_PATH`, `OPENSSL_PATH`
  and `CURL_PATH`.
- Image: `./scripts/build-and-deploy-image.sh --arch arm64`
- There are no unit tests.
