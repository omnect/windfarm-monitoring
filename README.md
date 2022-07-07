# windfarm-monitoring

## Instruction
This module is based on the ICS_DeviceManagement [iot-client-template-rs](https://github.com/ICS-DeviceManagement/iot-client-template-rs). All information you need to build the project can be found there.

## What is windfarm-monitoring
This module on the device side is designed to **demonstrate windfarm signal values** using Prometheus Metrics in a Rust Web Service:
The monitoring module provide the following metrics on his endpoint:
- latitude
- longitude
- wind_speed
- wind_direction

Default port endpoint is **8080**, but could be overritten by an os environment vairable **PORT**.


## License

Licensed under either of
* Apache License, Version 2.0, (./LICENSE-APACHE or <http://www.apache.org/licenses/LICENSE-2.0>)
* MIT license (./LICENSE-MIT or <http://opensource.org/licenses/MIT>)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms or
conditions.

Copyright (c) 2022 conplement AG