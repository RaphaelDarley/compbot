use std::env;

use sqlx::mysql::MySqlConnectOptions;
use sqlx::{ConnectOptions, MySqlConnection};

use crate::CSUser;

async fn get_conn() -> MySqlConnection {
    let db_pass = env::var("DB_PASSWORD").expect("Expected a database password in the environment");
    let db_port = env::var("DB_PORT")
        .ok()
        .map(|p| p.parse::<u16>().ok())
        .flatten()
        .unwrap_or(3306);

    MySqlConnectOptions::new()
        .host("localhost")
        .port(db_port)
        .username("compbot")
        .password(&db_pass)
        .database("oxcompsocnet")
        .connect()
        .await
        .unwrap()
}

pub async fn check_username(username: &str) -> bool {
    let mut conn = get_conn().await;
    let row = sqlx::query(
        r#"SELECT * FROM wp_postmeta WHERE meta_key = "_additional_wooccm5" AND meta_value = ?;"#,
    )
    .bind(username)
    .fetch_optional(&mut conn)
    .await
    .unwrap();

    row.is_some()
}

pub async fn email_lookup(email: &str) -> Option<CSUser> {
    let mut conn = get_conn().await;
    let row: Option<(u64, String, String)> = sqlx::query_as(
        r#"
    SELECT 
    email.post_id AS id,
    first_name.meta_value AS first_name,
    last_name.meta_value AS last_name
FROM 
    wp_postmeta email
JOIN 
    wp_postmeta first_name ON email.post_id = first_name.post_id AND first_name.meta_key = '_billing_first_name'
JOIN 
    wp_postmeta last_name ON email.post_id = last_name.post_id AND last_name.meta_key = '_billing_last_name'
WHERE 
    email.meta_key = '_billing_email'
    AND email.meta_value = ?;"#,
    ).bind(email).fetch_optional(&mut conn).await.unwrap();

    row.map(Into::into)
}
