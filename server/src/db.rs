use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use std::env;

pub async fn init_db() -> Result<Pool<Postgres>, sqlx::Error> {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
}

pub async fn save_user(pool: &Pool<Postgres>, email: &str, country: &str, state: Option<&str>) -> Result<(), sqlx::Error> {
    // Create table if not exists (simple approach for this task)
    // In production, use migrations.
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS waitlist (
            id SERIAL PRIMARY KEY,
            email TEXT NOT NULL UNIQUE,
            country TEXT NOT NULL,
            state TEXT,
            created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
        )
        "#
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "INSERT INTO waitlist (email, country, state) VALUES ($1, $2, $3) ON CONFLICT (email) DO NOTHING"
    )
    .bind(email)
    .bind(country)
    .bind(state)
    .execute(pool)
    .await?;
    Ok(())
}
