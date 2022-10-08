use salvo::{listener::TcpListener, Server};

use infra::{database, router};

mod app;
mod domain;
mod infra;

#[tokio::main]
async fn main() {
    let pool = database::create_pool().await;
    let listener = TcpListener::bind("127.0.0.1:3333");
    Server::new(listener).serve(router::app(pool)).await;
}
