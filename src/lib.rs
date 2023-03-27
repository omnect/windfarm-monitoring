pub mod client;
mod metrics_provider;
#[cfg(feature = "systemd")]
pub mod systemd;
pub mod twin;
use anyhow::Result;
use azure_iot_sdk::client::*;
use client::{Client, Message};
use log::{debug, error, warn};
use std::matches;
use std::sync::mpsc;
use std::sync::Once;
use twin::ReportProperty;

static INIT: Once = Once::new();

#[tokio::main]
pub async fn run() -> Result<()> {
    let mut client = Client::new();
    let (tx_client2app, rx_client2app) = mpsc::channel();
    let (tx_app2client, rx_app2client) = mpsc::channel();
    let twin = twin::get_or_init(Some(&tx_app2client));

    //client.run(None, None, tx_client2app, rx_app2client);
    client.run(Some("HostName=omnect-cp-dev-iot-hub.azure-devices.net;DeviceId=jza-ssh-test;ModuleId=omnect-device-service;SharedAccessKey=VlOFwDSZm7I6aL0dRx0KrlnsoDv4QBdPmLazOu5Hhgw="), None, tx_client2app, rx_app2client);

    for msg in rx_client2app {
        match msg {
            Message::Authenticated => {
                INIT.call_once(|| {
                    #[cfg(feature = "systemd")]
                    systemd::notify_ready();

                    twin.report(&ReportProperty::Versions)
                        .unwrap_or_else(|e| error!("report: {:#?}", e));
                });
            }
            Message::Unauthenticated(reason) => {
                anyhow::ensure!(
                    matches!(reason, UnauthenticatedReason::ExpiredSasToken),
                    "No connection. Reason: {:?}",
                    reason
                );
            }
            Message::Desired(state, desired) => {
                twin.update(state, desired)
                    .unwrap_or_else(|e| error!("update: {:#?}", e));
            }
            Message::C2D(msg) => {
                warn!(
                    "Received unexpected C2D message with \n body: {:?}\n properties: {:?} \n system properties: {:?}",
                    std::str::from_utf8(&msg.body).unwrap(),
                    msg.properties,
                    msg.system_properties
                );
            }
            _ => debug!("Application received unhandled message"),
        }
    }

    client.stop()
}
