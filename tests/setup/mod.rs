use lazy_static::lazy_static;
use reqwest::header::{HeaderMap, HeaderValue};

use std::{process::Command, time::Duration};

lazy_static! {
    static ref URL: url::Url = url::Url::parse("http://localhost:3333").unwrap();
}

pub fn test_url() -> &'static url::Url {
    &URL
}

fn handle_sqlx_cmd(cmd: &mut Command) {
    if cmd.spawn().unwrap().wait().unwrap().success() {
        panic!("Could not exec sqlx command to setup the database");
    }
}

pub fn setup_database() {
    dotenv::dotenv().unwrap();

    let host = std::env::var("DATABASE_HOST").unwrap();
    let db_name = std::env::var("DATABASE_NAME").unwrap();
    let user = std::env::var("DATABASE_USER").unwrap();
    let password = std::env::var("DATABASE_PASSWORD").unwrap();
    let port: u16 = std::env::var("DATABASE_PORT")
        .unwrap()
        .parse()
        .expect("Invalid DATABASE_PORT");
    let database_url = format!("postgresql://{user}:{password}@{host}:{port}/{db_name}");

    // sqlx database drop --database-url DATABASE_URL -y
    handle_sqlx_cmd(
        Command::new("sqlx")
            .arg("database")
            .arg("drop")
            .arg("--database-url")
            .arg(&database_url)
            .arg("-y"),
    );

    // sqlx database create --database-url DATABASE_URL
    handle_sqlx_cmd(
        Command::new("sqlx")
            .arg("database")
            .arg("create")
            .arg("--database-url")
            .arg(&database_url),
    );

    // sqlx migrate run --database-url DATABASE_URL
    handle_sqlx_cmd(
        Command::new("sqlx")
            .arg("migrate")
            .arg("run")
            .arg("--database-url")
            .arg(&database_url),
    );
}

pub fn create_client() -> reqwest::Client {
    let mut headers = HeaderMap::new();
    headers.append("accept", HeaderValue::from_static("application/json"));

    let keep_alive = 1000 * 60 * 60; // 1 hours
    let connect_timeout = 1000 * 5; // 5 sec
    let timeout = 1000 * 10; // 10 sec

    let client = reqwest::Client::builder()
        .tcp_keepalive(Duration::from_millis(keep_alive))
        .connect_timeout(Duration::from_millis(connect_timeout))
        .timeout(Duration::from_millis(timeout))
        .pool_max_idle_per_host(5)
        .default_headers(headers)
        .brotli(true)
        .gzip(true)
        .build()
        .expect("could not create client");
    client
}
