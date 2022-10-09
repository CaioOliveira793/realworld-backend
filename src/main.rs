use salvo::{listener::TcpListener, Server};

use config::env_var;
use infra::{database, router};

mod app;
mod config;
mod domain;
mod infra;

#[tokio::main]
async fn main() {
    let pool = database::create_pool().await;
    let address = format!("0.0.0.0:{}", env_var::get().port);
    let listener = TcpListener::bind(&address);
    Server::new(listener).serve(router::app(pool)).await;
}
