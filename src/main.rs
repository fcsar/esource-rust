// Ativa warnings extras do clippy para o projeto inteiro
#![warn(clippy::pedantic)]
// Permite alguns que são restritivos demais pra início de projeto
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]

#[tokio::main]
async fn main() {
    // Inicializa o sistema de logs
    tracing_subscriber::fmt()
        .with_env_filter("edriven=debug,info")
        .init();

    tracing::info!("edriven ledger starting...");
}
