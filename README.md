# windfarm-monitoring
Product page: https://www.omnect.io/home

# What is windfarm-monitoring
This module is designed to **demonstrate windfarm signal values** and provides the following metrics via an D2C(device-to-cloud) message:

- latitude
- longitude
- wind_speed
- wind_direction

The device-to-cloud message will be transmitted to edgeHub module on an **edge device** and must be manual routed to the IotHub with the following deployment route:
**FROM /messages/modules/windfarm-monitoring/outputs/metrics INTO $upstream**

The message protocol used is as follows:

```
{
    "body": [
        {
            "TimeGeneratedUtc": "2024-10-25T11:19:17.0339573Z",
            "Name": "latitude",
            "Value": 53.91164304155669,
            "Labels": {
                "edge_device": "test-device",
                "iothub": "test-iot-hub.azure-devices.net",
                "module_name": "windfarm-monitoring"
            }
        },
        {
            "TimeGeneratedUtc": "2024-10-25T11:19:17.0339573Z",
            "Name": "longitude",
            "Value": 8.6557029326582,
            "Labels": {
                "edge_device": "test-device",
                "iothub": "test-iot-hub.azure-devices.net",
                "module_name": "windfarm-monitoring"
            }
        },
        {
            "TimeGeneratedUtc": "2024-10-25T11:19:17.0339573Z",
            "Name": "wind_direction",
            "Value": 130,
            "Labels": {
                "edge_device": "test-device",
                "iothub": "test-iot-hub.azure-devices.net",
                "module_name": "windfarm-monitoring"
            }
        },
        {
            "TimeGeneratedUtc": "2024-10-25T11:19:17.0339573Z",
            "Name": "wind_speed",
            "Value": 6.545120016746934,
            "Labels": {
                "edge_device": "test-device",
                "iothub": "test-iot-hub.azure-devices.net",
                "module_name": "windfarm-monitoring"
            }
        }
    ]
}
```



# License
Licensed under either of
* Apache License, Version 2.0, (./LICENSE-APACHE or <http://www.apache.org/licenses/LICENSE-2.0>)
* MIT license (./LICENSE-MIT or <http://opensource.org/licenses/MIT>)
at your option.

# Contribution
Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms or
conditions.


---

copyright (c) 2022 conplement AG<br>
Content published under the Apache License Version 2.0 or MIT license, are marked as such. They may be used in accordance with the stated license conditions.
