use crate::metrics_provider::MetricsProvider;
use anyhow::Result;
use azure_iot_sdk::client::*;
use log::{error, info};
use rand::{thread_rng, Rng};
use serde_json::json;
use tokio::{select, sync::mpsc};

pub struct Twin {
    iothub_client: IotHubClient,
    authenticated_once: bool,
    location_once: bool,
    metrics_provider: MetricsProvider,
    tx_reported_properties: mpsc::Sender<serde_json::Value>,
    rx_reported_properties: mpsc::Receiver<serde_json::Value>,
    tx_outgoing_message: mpsc::Sender<IotMessage>,
    rx_outgoing_message: mpsc::Receiver<IotMessage>,
}

impl Twin {
    pub fn new(client: IotHubClient) -> Self {
        let (tx_reported_properties, rx_reported_properties) = mpsc::channel(100);
        let (tx_outgoing_message, rx_outgoing_message) = mpsc::channel(100);
        Twin {
            iothub_client: client,
            tx_reported_properties,
            rx_reported_properties,
            tx_outgoing_message,
            rx_outgoing_message,
            authenticated_once: false,
            location_once: false,
            metrics_provider: MetricsProvider::new(),
        }
    }

    async fn handle_connection_status(&mut self, auth_status: AuthenticationStatus) -> Result<()> {
        info!("auth_status: {auth_status:#?}");

        match auth_status {
            AuthenticationStatus::Authenticated => {
                if !self.authenticated_once {
                    self.authenticated_once = true;

                    self.tx_reported_properties
                        .send(json!({
                            "module-version": env!("CARGO_PKG_VERSION"),
                            "azure-sdk-version": IotHubClient::sdk_version_string()
                        }))
                        .await?;
                };
            }
            AuthenticationStatus::Unauthenticated(reason) => {
                anyhow::ensure!(
                    matches!(reason, UnauthenticatedReason::ExpiredSasToken),
                    "No connection. Reason: {reason:?}"
                );
            }
        }

        Ok(())
    }

    async fn handle_desired(
        &mut self,
        state: TwinUpdateState,
        desired: serde_json::Value,
    ) -> Result<()> {
        info!("desired: {state:#?}, {desired}");

        let coordinates = desired["reported"]["location"].as_object();
        if !self.location_once {
            let location = match coordinates {
                Some(values) => json!({ "location": values }),
                _ => json!({
                    "location": {
                        "latitude": thread_rng().gen_range(53.908754f64..53.956915f64),
                        "longitude": thread_rng().gen_range(8.594901f64..8.741848f64)
                    }
                }),
            };

            self.tx_reported_properties.send(location.clone()).await?;

            self.metrics_provider.run(
                self.tx_outgoing_message.clone(),
                location["location"].clone(),
            );

            self.location_once = true;
        };
        Ok(())
    }

    pub async fn run() -> Result<()> {
        let (tx_connection_status, mut rx_connection_status) = mpsc::channel(100);
        let (tx_twin_desired, mut rx_twin_desired) = mpsc::channel(100);

        let mut twin = Self::new(
            IotHubClient::builder()
                .observe_connection_state(tx_connection_status)
                .observe_desired_properties(tx_twin_desired)
                .build_edge_client()?,
        );

        loop {
            select! (
                status = rx_connection_status.recv() => {
                    twin.handle_connection_status(status.unwrap()).await?;
                },
                desired = rx_twin_desired.recv() => {
                    let desired = desired.unwrap();
                    twin.handle_desired(desired.state, desired.value)
                        .await
                        .unwrap_or_else(|e| error!("twin update desired properties: {e:#}"));
                },
                reported = twin.rx_reported_properties.recv() => {
                    twin.iothub_client.twin_report(reported.unwrap())?
                },
                Some(message) = twin.rx_outgoing_message.recv() => {
                    twin.iothub_client
                        .send_d2c_message(message)?
                },
            );
        }
    }
}
