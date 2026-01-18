use axum::{
    extract::{State, Json},
    http::StatusCode,
    response::IntoResponse,
    routing::{post},
    Router,
};
use serde::Deserialize;
use std::net::SocketAddr;
use tower_http::{
    cors::CorsLayer,
    services::ServeDir,
    trace::TraceLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use std::sync::Arc;

mod db;
mod email;

struct AppState {
    db: sqlx::Pool<sqlx::Postgres>,
}

#[derive(Deserialize)]
struct JoinWaitlistRequest {
    email: String,
    country: String,
    state: Option<String>,
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "server=debug,tower_http=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Initializing Database Connection...");
    let pool = db::init_db().await.expect("Failed to connect to DB");
    
    let state = Arc::new(AppState { db: pool });

    // Serve the 'dist' directory which contains the built Astro site
    // We assume the binary is run from the project root or server dir, but 'dist' is in project root.
    // If running from server/ dir, it is ../dist
    // If running from project root, it is ./dist
    // Let's check where the user will run it. Likely from compiled binary or docker.
    // In Docker, we will copy dist to a known location.
    
    let dist_path = std::env::var("STATIC_DIR").unwrap_or_else(|_| "../dist".to_string());
    tracing::info!("Serving static files from: {}", dist_path);
    
    let static_files = ServeDir::new(dist_path).append_index_html_on_directories(true);

    let app = Router::new()
        .route("/api/join-waitlist", post(join_waitlist))
        .fallback_service(static_files)
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state);

    let port = std::env::var("PORT").unwrap_or("3000".to_string()).parse().unwrap();
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    
    tracing::info!("listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn join_waitlist(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<JoinWaitlistRequest>,
) -> impl IntoResponse {
    let email = payload.email.clone();
    let country = payload.country.clone();
    let user_state = payload.state.clone();
    
    tracing::info!("Received join request for: {}", email);

    // 1. Save to DB
    if let Err(e) = db::save_user(&state.db, &email, &country, user_state.as_deref()).await {
        tracing::error!("Database error: {:?}", e);
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "Failed to save user"}))).into_response();
    }

    // 2. Process Brevo (Add contact + Send Email)
    tokio::spawn(async move {
        // Add to contacts list (Enables automation/follow-ups)
        if let Err(e) = email::add_contact_to_brevo(&email, &country, state.as_deref()).await {
            tracing::error!("Failed to add contact to Brevo {}: {:?}", email, e);
        }

        // Send immediate confirmation email
        if let Err(e) = email::send_confirmation_email(&email).await {
            tracing::error!("Failed to send email to {}: {:?}", email, e);
        } else {
            tracing::info!("Confirmation email sent to {}", email);
        }
    });

    (StatusCode::OK, Json(serde_json::json!({"message": "Success"}))).into_response()
}
