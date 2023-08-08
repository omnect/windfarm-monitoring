pub mod twin;
mod metrics_provider;

use azure_iot_sdk::client::*;
use env_logger::{Builder, Env};
use log::{error, info};
use std::process;
use twin::Twin;

#[tokio::main]
async fn main() {
    log_panics::init();

    if cfg!(debug_assertions) {
        Builder::from_env(Env::default().default_filter_or("debug")).init();
    } else {
        Builder::from_env(Env::default().default_filter_or("info")).init();
    }

    info!("module version: {}", env!("CARGO_PKG_VERSION"),);
    info!("azure sdk version: {}", IotHubClient::sdk_version_string());

    if let Err(e) = Twin::run().await {
        error!("Application error: {e:#?}");

        process::exit(1);
    }
}
