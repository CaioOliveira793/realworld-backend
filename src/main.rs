use std::net::SocketAddrV4;

use salvo::{listener::TcpListener, logging::Logger, Server};
use tokio::signal::ctrl_c;

use config::env_var;
use infra::{database, router};

mod app;
mod config;
mod domain;
mod infra;

async fn handle_shutdown() {
    match ctrl_c().await {
        Ok(_) => return,
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

    let pool = database::create_pool().await;
    let address = SocketAddrV4::new([0, 0, 0, 0].into(), env_var::get().port);
    let router = router::app(pool).hoop(Logger);
    let listener = TcpListener::bind(&address);
    Server::new(listener)
        .serve_with_graceful_shutdown(router, handle_shutdown())
        .await;
}
