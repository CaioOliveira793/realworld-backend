#![warn(clippy::unwrap_used)]

use std::net::SocketAddrV4;

use domain::service::Argon2HashService;
use salvo::{listener::TcpListener, logging::Logger, Server};
use tokio::signal::ctrl_c;

use config::env_var;
use infra::{database, router};

mod app;
mod base;
mod config;
mod domain;
mod error;
mod infra;

async fn handle_shutdown() {
    match ctrl_c().await {
        Ok(_) => (),
        Err(err) => {
            tracing::error!(
                target = "conduit::error::signal",
                cause = %err,
            );
        }
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let pool = database::connection::create_sqlx_pool().await;
    let hash_service = Argon2HashService::new();

    let address = SocketAddrV4::new([0, 0, 0, 0].into(), env_var::get().port);
    let router = router::app(pool, hash_service).hoop(Logger);
    let listener = TcpListener::bind(&address);
    Server::new(listener)
        .serve_with_graceful_shutdown(router, handle_shutdown())
        .await;
}
