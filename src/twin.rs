use crate::metrics_provider::MetricsProvider;
use anyhow::Result;
use azure_iot_sdk::client::*;
use log::{error, info, warn};
use rand::Rng;
use serde_json::json;
use std::time::{self, Duration};
use tokio::{select, sync::mpsc};

#[derive(PartialEq)]
enum TwinState {
    Uninitialized,
    Initialized,
    Authenticated,
}

pub struct Twin {
    client: Option<IotHubClient>,
    state: TwinState,
    location_once: bool,
    metrics_provider: MetricsProvider,
    tx_reported_properties: mpsc::Sender<serde_json::Value>,
    tx_outgoing_message: mpsc::Sender<IotMessage>,
}

impl Twin {
    fn new(
        tx_reported_properties: mpsc::Sender<serde_json::Value>,
        tx_outgoing_message: mpsc::Sender<IotMessage>,
    ) -> Self {
        Twin {
            client: None,
            state: TwinState::Uninitialized,
            location_once: false,
            metrics_provider: MetricsProvider::new(),
            tx_reported_properties,
            tx_outgoing_message,
        }
    }

    async fn connect_twin(&mut self) -> Result<()> {
        self.tx_reported_properties
            .send(json!({
                "module-version": env!("CARGO_PKG_VERSION"),
                "azure-sdk-version": IotHubClient::sdk_version_string()
            }))
            .await
            .map_err(Into::into)
    }

    async fn reset_client_with_delay(&mut self, timeout: Option<time::Duration>) {
        if let Some(client) = self.client.as_mut() {
            info!("reset_client: shutdown iotclient");
            client.shutdown(Duration::from_secs(5)).await;
            self.client = None;
        }
        if let Some(t) = timeout {
            info!("reset_client: sleep for {}ms", t.as_millis());
            tokio::time::sleep(t).await;
        }
    }

    async fn connect_iothub_client(builder: &IotHubClientBuilder) -> Result<IotHubClient> {
        info!("connecting to iothub...");
        builder.build_edge_client()
    }

    async fn handle_connection_status(
        &mut self,
        auth_status: AuthenticationStatus,
    ) -> Result<bool> {
        let mut restart_twin = false;

        match auth_status {
            AuthenticationStatus::Authenticated => {
                if self.state != TwinState::Authenticated {
                    info!("succeeded to connect to iothub");
                    self.connect_twin().await?;
                    self.state = TwinState::Authenticated;
                }
            }
            AuthenticationStatus::Unauthenticated(reason) => {
                if self.state == TwinState::Authenticated {
                    self.state = TwinState::Initialized;
                }

                match reason {
                    UnauthenticatedReason::BadCredential
                    | UnauthenticatedReason::CommunicationError => {
                        error!(
                            "failed to connect to iothub: {reason:?}. Possible reasons: certificate renewal or wrong system time"
                        );
                        restart_twin = true;
                    }
                    UnauthenticatedReason::RetryExpired
                    | UnauthenticatedReason::ExpiredSasToken
                    | UnauthenticatedReason::NoNetwork
                    | UnauthenticatedReason::Unknown => {
                        info!("iothub connection lost: {reason:?}");
                    }
                    UnauthenticatedReason::DeviceDisabled => {
                        warn!("iothub connection lost: {reason:?}");
                    }
                }
            }
        }

        Ok(restart_twin)
    }

    async fn handle_desired(
        &mut self,
        state: TwinUpdateState,
        desired: serde_json::Value,
    ) -> Result<()> {
        info!("desired: {state:#?}, {desired}");

        let coordinates = match state {
            TwinUpdateState::Complete => desired["desired"]["location"].as_object(),
            TwinUpdateState::Partial => desired["location"].as_object(),
        };

        if !self.location_once {
            let location = match coordinates {
                Some(values) => json!({ "location": values }),
                _ => json!({
                    "location": {
                        "latitude": rand::rng().random_range(53.908754f64..53.956915f64),
                        "longitude": rand::rng().random_range(8.594901f64..8.741848f64)
                    }
                }),
            };

            self.tx_reported_properties.send(location.clone()).await?;

            self.metrics_provider.run(
                self.tx_outgoing_message.clone(),
                location["location"].clone(),
            );

            self.location_once = true;
        }

        Ok(())
    }

    pub async fn run() -> Result<()> {
        let (tx_connection_status, mut rx_connection_status) = mpsc::channel(100);
        let (tx_twin_desired, mut rx_twin_desired) = mpsc::channel(100);
        let (tx_reported_properties, mut rx_reported_properties) = mpsc::channel(100);
        let (tx_outgoing_message, mut rx_outgoing_message) = mpsc::channel(100);

        let mut twin = Self::new(tx_reported_properties, tx_outgoing_message);

        let client_builder = IotHubClient::builder()
            .observe_connection_state(tx_connection_status)
            .observe_desired_properties(tx_twin_desired);

        tokio::pin! {
            let client_created = Self::connect_iothub_client(&client_builder);
        }

        loop {
            select! {
                biased;

                result = &mut client_created, if twin.client.is_none() => {
                    match result {
                        Ok(client) => {
                            info!("iothub client created");
                            twin.client = Some(client);
                        }
                        Err(e) => {
                            error!("couldn't create iothub client: {e:#}");
                            twin.reset_client_with_delay(Some(time::Duration::from_secs(10))).await;
                            client_created.set(Self::connect_iothub_client(&client_builder));
                        }
                    }
                },
                Some(status) = rx_connection_status.recv() => {
                    if twin.handle_connection_status(status).await? {
                        twin.reset_client_with_delay(Some(time::Duration::from_secs(1))).await;
                        client_created.set(Self::connect_iothub_client(&client_builder));
                    }
                },
                desired = rx_twin_desired.recv() => {
                    let desired = desired.unwrap();
                    twin.handle_desired(desired.state, desired.value)
                        .await
                        .unwrap_or_else(|e| error!("twin update desired properties: {e:#}"));
                },
                Some(reported) = rx_reported_properties.recv() => {
                    let Some(client) = &twin.client else {
                        error!("couldn't report properties since client not present");
                        continue
                    };
                    client.twin_report(reported)?
                },
                Some(message) = rx_outgoing_message.recv() => {
                    let Some(client) = &twin.client else {
                        error!("couldn't send msg since client not present");
                        continue
                    };
                    client.send_d2c_message(message)?
                },
            }
        }
    }
}
