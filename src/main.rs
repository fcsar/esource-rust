#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("edriven=debug,info")
        .init();

    tracing::info!("edriven ledger starting...");
}
