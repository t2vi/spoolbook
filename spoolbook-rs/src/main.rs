use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::str::FromStr;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    let options = SqliteConnectOptions::from_str("sqlite://dev.db?mode=rwc")
        .unwrap()
        .foreign_keys(true);
    let pool = SqlitePoolOptions::new()
        .connect_with(options)
        .await
        .expect("failed to open dev.db");
    sqlx::migrate!().run(&pool).await.expect("migration failed");

    let app = spoolbook_rs::app(pool);

    let listener = TcpListener::bind("127.0.0.1:8090").await.unwrap();
    println!("spoolbook-rs listening on http://127.0.0.1:8090");
    axum::serve(listener, app).await.unwrap();
}
