use crate::metrics_provider::MetricsProvider;
use anyhow::{anyhow, Context, Result};
use azure_iot_sdk::client::*;
use log::{error, info};
use rand::{thread_rng, Rng};
use serde_json::json;
use tokio::{
    select,
    sync::mpsc,
    time::{timeout, Duration},
};

pub struct Twin {
    iothub_client: Box<dyn IotHub>,
    authenticated_once: bool,
    location_once: bool,
    metrics_provider: MetricsProvider,
    tx_reported_properties: mpsc::Sender<serde_json::Value>,
}

impl Twin {
    pub fn new(
        client: Box<dyn IotHub>,
        tx_reported_properties: mpsc::Sender<serde_json::Value>,
    ) -> Self {
        Twin {
            iothub_client: client,
            tx_reported_properties,
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

            self.metrics_provider.run(location["location"].clone());

            self.location_once = true;
        };
        Ok(())
    }

    async fn handle_report_property(&mut self, properties: serde_json::Value) -> Result<()> {
        info!("report: {properties}");

        match timeout(
            Duration::from_secs(5),
            self.iothub_client.twin_report(properties),
        )
        .await
        {
            Ok(result) => result.context("handle_report_property: couldn't report property"),
            Err(_) => Err(anyhow!("handle_report_property: timeout occured")),
        }
    }

    pub async fn run() -> Result<()> {
        let (tx_connection_status, mut rx_connection_status) = mpsc::channel(100);
        let (tx_twin_desired, mut rx_twin_desired) = mpsc::channel(100);
        let (tx_reported_properties, mut rx_reported_properties) = mpsc::channel(100);

        let client = IotHubClient::from_edge_environment(
            Some(tx_connection_status.clone()),
            Some(tx_twin_desired.clone()),
            None,
            None,
        )?;

        let mut twin = Self::new(client, tx_reported_properties);

        loop {
            select! (
                status = rx_connection_status.recv() => {
                    twin.handle_connection_status(status.unwrap()).await?;
                },
                desired = rx_twin_desired.recv() => {
                    let (state, desired) = desired.unwrap();
                    twin.handle_desired(state, desired).await.unwrap_or_else(|e| error!("twin update desired properties: {e:#?}"));
                },
                reported = rx_reported_properties.recv() => {
                    twin.handle_report_property(reported.unwrap()).await?;
                },
            );
        }
    }
}
