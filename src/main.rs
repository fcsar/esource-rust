#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]

mod accounting;
mod shared;

use std::sync::Arc;

use axum::routing::post;
use axum::Router;
use sqlx::postgres::PgPoolOptions;
use tokio::net::TcpListener;

use crate::accounting::application::command_handler::CommandHandler;
use crate::accounting::application::query_handler::QueryHandler;
use crate::accounting::infra::http::{handlers, queries};
use crate::accounting::infra::pg::event_store::PgEventStore;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("edriven=debug,info")
        .init();

    tracing::info!("edriven ledger starting...");
    dotenvy::dotenv().ok();

    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL must be set in .env");

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await
        .expect("failed to connect to database");

    tracing::info!("connected to database");

    let worker_pool = pool.clone();
    tokio::spawn(accounting::worker::projection::run(worker_pool));

    let query_pool = pool.clone();
    let store = PgEventStore::new(pool);
    let command_state = Arc::new(CommandHandler::new(store));
    let query_state = Arc::new(QueryHandler::new(query_pool));

    let command_routes = Router::new()
        .route("/accounts", post(handlers::create_account))
        .route("/accounts/{id}/deposit", post(handlers::deposit))
        .route("/accounts/{id}/withdraw", post(handlers::withdraw))
        .with_state(command_state);

    let query_routes = Router::new()
        .route("/accounts/{id}/balance", axum::routing::get(queries::get_balance))
        .route("/accounts/{id}/statement", axum::routing::get(queries::get_statement))
        .route("/replay", post(queries::replay))
        .with_state(query_state);

    let app = command_routes.merge(query_routes);

    let listener = TcpListener::bind("0.0.0.0:3000").await.unwrap();
    tracing::info!("listening on http://localhost:3000");

    axum::serve(listener, app).await.unwrap();
}
