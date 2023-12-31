use axum::routing::{get, post};
use axum::Router;
use fq::{auth, routes, ws};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tower_cookies::CookieManagerLayer;
use tower_http::services::ServeDir;
// use tower_http::trace::{DefaultMakeSpan, TraceLayer};
use fq::app_state::AppState;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

const PORT: u16 = 4000;
const DB_FILENAME: &str = "fq.db";

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "fq=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // let span = DefaultMakeSpan::default().include_headers(true);
    // let trace_layer = TraceLayer::new_for_http().make_span_with(span);

    let cwd = std::env::current_dir().unwrap();
    let assets_path = format!("{}/assets", cwd.to_str().unwrap());
    let assets_service = ServeDir::new(assets_path);

    let pool = fq::create_db_pool(DB_FILENAME).await.unwrap();
    let app_state = Arc::new(AppState {
        pool,
        rooms: Mutex::new(HashMap::new()),
    });

    let router = Router::new()
        .route("/", get(routes::index_page))
        .route("/ws", get(ws::upgrade_ws))
        .route("/auth", get(auth::authorize))
        .route("/user", get(routes::user_page))
        .route("/user/name", post(routes::update_username))
        .route("/games", post(routes::handle_game_create))
        .route("/games/:id", get(routes::game_page))
        // .route("/games/:id/start", post(routes::handle_game_start))
        .nest_service("/assets", assets_service)
        .layer(CookieManagerLayer::new())
        // .layer(trace_layer)
        .with_state(app_state);

    let addr = SocketAddr::from(([127, 0, 0, 1], PORT));
    tracing::debug!("listening on {}", addr);

    axum::Server::bind(&addr)
        .serve(router.into_make_service_with_connect_info::<SocketAddr>())
        .await
        .unwrap();
}

// let error_layer = HandleErrorLayer::new(|_: BoxError| async { StatusCode::BAD_REQUEST });
//
// let session_store = SqliteStore::new(pool.clone());
// session_store.migrate().await.unwrap();
//
// let session_layer = SessionManagerLayer::new(session_store)
//     .with_secure(false)
//     .with_max_age(time::Duration::days(1));
//
// let session_service = ServiceBuilder::new()
//     .layer(error_layer)
//     .layer(session_layer);
