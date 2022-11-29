use reqwest::{
    header::{HeaderMap, HeaderValue},
    Client,
};
use url::Url;

use std::time::Duration;

pub async fn setup_test() -> (Client, Url, sqlx::PgPool) {
    dotenv::dotenv().unwrap();
    (create_client(), service_url(), setup_database().await)
}

fn service_url() -> Url {
    let port: u16 = std::env::var("PORT")
        .unwrap()
        .parse()
        .expect("Invalid PORT");
    Url::parse(format!("http://localhost:{port}").as_str()).unwrap()
}

async fn setup_database() -> sqlx::PgPool {
    let database_host = std::env::var("DATABASE_HOST").unwrap();
    let database_name = std::env::var("DATABASE_NAME").unwrap();
    let database_user = std::env::var("DATABASE_USER").unwrap();
    let database_password = std::env::var("DATABASE_PASSWORD").unwrap();
    let database_port: u16 = std::env::var("DATABASE_PORT")
        .unwrap()
        .parse()
        .expect("Invalid DATABASE_PORT");

    let database_url = format!("postgres://{database_user}:{database_password}@{database_host}:{database_port}/{database_name}");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .min_connections(1)
        .max_connections(5)
        .acquire_timeout(Duration::from_millis(1000))
        .idle_timeout(Duration::from_millis(1000 * 30))
        .max_lifetime(Duration::from_millis(1000 * 10))
        .connect(&database_url)
        .await
        .expect("Expect to create a database pool with a open connection");

    let drop_sttm = sqlx::query("DROP SCHEMA IF EXISTS iam, blog CASCADE");

    let mut trx = pool.begin().await.unwrap();
    drop_sttm.execute(&mut trx).await.unwrap();
    for sttm in include_str!("../../dbschema.sql").split(';') {
        sqlx::query(sttm).execute(&mut trx).await.unwrap();
    }
    trx.commit().await.unwrap();

    pool
}

fn create_client() -> reqwest::Client {
    let mut headers = HeaderMap::new();
    headers.append("accept", HeaderValue::from_static("application/json"));

    let keep_alive = 1000 * 60 * 60; // 1 hours
    let connect_timeout = 1000 * 5; // 5 sec
    let timeout = 1000 * 10; // 10 sec

    reqwest::Client::builder()
        .tcp_keepalive(Duration::from_millis(keep_alive))
        .connect_timeout(Duration::from_millis(connect_timeout))
        .timeout(Duration::from_millis(timeout))
        .pool_max_idle_per_host(5)
        .default_headers(headers)
        .brotli(true)
        .gzip(true)
        .build()
        .expect("Expect to create a http client")
}
