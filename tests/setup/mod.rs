use pg_pool::{Config, ManagerConfig, Pool, RecyclingMethod, Runtime, SslMode};
use pg_tokio::NoTls;
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Client,
};
use url::Url;

use std::time::Duration;

pub async fn setup_test() -> (Client, Url, Pool) {
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

async fn setup_database() -> pg_pool::Pool {
    let host = std::env::var("DATABASE_HOST").unwrap();
    let db_name = std::env::var("DATABASE_NAME").unwrap();
    let user = std::env::var("DATABASE_USER").unwrap();
    let password = std::env::var("DATABASE_PASSWORD").unwrap();
    let port: u16 = std::env::var("DATABASE_PORT")
        .unwrap()
        .parse()
        .expect("Invalid DATABASE_PORT");
    let mut cfg = Config::new();
    cfg.host = Some(host);
    cfg.dbname = Some(db_name);
    cfg.port = Some(port);
    cfg.user = Some(user);
    cfg.password = Some(password);
    cfg.manager = Some(ManagerConfig {
        recycling_method: RecyclingMethod::Fast,
    });
    cfg.ssl_mode = Some(SslMode::Prefer);

    let pool = cfg
        .create_pool(Some(Runtime::Tokio1), NoTls)
        .expect("should create a connection pool");

    let mut client = pool
        .get()
        .await
        .expect("should retrieve database client from pool");

    let trx = client.transaction().await.unwrap();
    trx.query("DROP SCHEMA IF EXISTS iam, blog CASCADE", &[])
        .await
        .unwrap();
    trx.batch_execute(include_str!("../../dbschema.sql"))
        .await
        .unwrap();
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
