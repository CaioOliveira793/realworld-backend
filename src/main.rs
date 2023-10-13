#![warn(clippy::unwrap_used)]

use std::{net::SocketAddr, sync::Arc};

use salvo::{listener::TcpListener, Server};
use tokio::signal::ctrl_c;

use config::env_var;
use infra::{
    database, router,
    service::{Argon2HashService, JWTEncryptionService},
};

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
                target = "shutdown::signal",
                cause = %err,
            );
        }
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let pool = database::connection::create_sqlx_pool().await;

    let addr: SocketAddr = ([0, 0, 0, 0], env_var::get().port).into();
    let router = router::app(
        &pool,
        Arc::new(Argon2HashService::new()),
        Arc::new(JWTEncryptionService::from_config()),
    );
    let listener = TcpListener::bind(&addr);
    Server::new(listener)
        .serve_with_graceful_shutdown(router, handle_shutdown())
        .await;

    tracing::info_span!("shutdown::database")
        .in_scope(|| async {
            pool.close().await;
        })
        .await;
}
