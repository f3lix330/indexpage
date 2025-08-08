use axum::{
    extract::{Path, State},
    routing::{get, post, delete},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::net::SocketAddr;
use dotenvy::dotenv;
use std::env;

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
struct Service {
    id: i32,
    name: String,
    link: String,
}

#[derive(Debug, Deserialize)]
struct CreateService {
    name: String,
    link: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    let database_url = env::var("DATABASE_URL")?;

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    // Ensure table exists
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS services (
            id SERIAL PRIMARY KEY,
            name TEXT UNIQUE NOT NULL,
            link TEXT UNIQUE NOT NULL
        )
        "#
    )
    .execute(&pool)
    .await?;

    let app = Router::new()
        .route("/services", get(get_services).post(create_service))
        .route("/services/:name", delete(delete_service))
        .with_state(pool);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Server running at http://{}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}

// GET /services
async fn get_services(State(pool): State<PgPool>) -> Json<Vec<Service>> {
    let services = sqlx::query_as::<_, Service>("SELECT * FROM services")
        .fetch_all(&pool)
        .await
        .unwrap_or_else(|_| vec![]);
    Json(services)
}

// POST /services
async fn create_service(
    State(pool): State<PgPool>,
    Json(payload): Json<CreateService>,
) -> Result<Json<Service>, (axum::http::StatusCode, String)> {
    let result = sqlx::query_as::<_, Service>(
        "INSERT INTO services (name, link) VALUES ($1, $2) RETURNING *",
    )
    .bind(&payload.name)
    .bind(&payload.link)
    .fetch_one(&pool)
    .await;

    match result {
        Ok(service) => Ok(Json(service)),
        Err(e) => Err((
            axum::http::StatusCode::BAD_REQUEST,
            format!("Failed to insert: {}", e),
        )),
    }
}

// DELETE /services/:name
async fn delete_service(
    State(pool): State<PgPool>,
    Path(name): Path<String>,
) -> Result<String, (axum::http::StatusCode, String)> {
    let result = sqlx::query("DELETE FROM services WHERE name = $1")
        .bind(&name)
        .execute(&pool)
        .await;

    match result {
        Ok(r) if r.rows_affected() > 0 => Ok(format!("Deleted '{}'", name)),
        Ok(_) => Err((axum::http::StatusCode::NOT_FOUND, "Service not found".into())),
        Err(e) => Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}