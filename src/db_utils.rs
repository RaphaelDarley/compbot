use std::env;

use sqlx::mysql::MySqlConnectOptions;
use sqlx::ConnectOptions;

pub async fn check_username(username: &str) -> bool {
    let db_pass = env::var("DB_PASSWORD").expect("Expected a database password in the environment");
    let db_port = env::var("DB_PORT")
        .ok()
        .map(|p| p.parse::<u16>().ok())
        .flatten()
        .unwrap_or(3306);

    let mut conn = MySqlConnectOptions::new()
        .host("localhost")
        .port(db_port)
        .username("compbot")
        .password(&db_pass)
        .database("oxcompsocnet")
        .connect()
        .await
        .unwrap();

    let row = sqlx::query(
        r#"SELECT * FROM wp_postmeta WHERE meta_key = "_additional_wooccm5" AND meta_value = ?;"#,
    )
    .bind(username)
    .fetch_optional(&mut conn)
    .await
    .unwrap();

    // println!("user found: {}", row.is_some());
    row.is_some()
}
