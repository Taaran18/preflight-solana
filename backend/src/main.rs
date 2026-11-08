mod ai;
mod config;
mod error;
mod guard;
mod inspect;
mod intent;
mod jupiter;
mod pipeline;
mod portfolio;
mod risk;
mod routes;
mod rpc;
mod simulate;
mod solana;
mod stream;
mod time;
mod tokens;
mod types;

use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use axum::{
    Router,
    http::{HeaderValue, Method, header::CONTENT_TYPE},
    routing::{get, post},
};
use tokio::sync::Semaphore;
use tower_http::cors::{AllowOrigin, CorsLayer};

use crate::{config::Config, jupiter::Jupiter, rpc::Rpc, types::Portfolio};

pub struct AppState {
    pub cfg: Config,
    pub http: reqwest::Client,
    pub rpc: Rpc,
    pub jup: Jupiter,
    pub portfolios: Mutex<HashMap<String, (Instant, Portfolio)>>,
    pub limiter: Arc<Semaphore>,
    pub market: Mutex<Option<(Instant, types::MarketResponse)>>,
}

pub type Shared = Arc<AppState>;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();
    let cfg = Config::from_env();
    let http = reqwest::Client::builder()
        .timeout(Duration::from_secs(120))
        .connect_timeout(Duration::from_secs(10))
        .build()?;
    let rpc = Rpc::new(http.clone(), cfg.rpc_url.clone());
    let jup = Jupiter::new(http.clone(), &cfg);
    let port = cfg.port;
    let host = cfg.host;
    cfg.print_summary();

    let origins: Vec<HeaderValue> = cfg.allowed_origins.iter().filter_map(|origin| origin.parse().ok()).collect();
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([CONTENT_TYPE]);

    let state: Shared = Arc::new(AppState {
        cfg,
        http,
        rpc,
        jup,
        portfolios: Mutex::new(HashMap::new()),
        limiter: Arc::new(Semaphore::new(4)),
        market: Mutex::new(None),
    });

    let app = Router::new()
        .route("/api/health", get(routes::health))
        .route("/api/portfolio", get(routes::portfolio))
        .route("/api/plan", post(routes::plan))
        .route("/api/inspect", post(routes::inspect))
        .route("/api/sample", post(routes::sample))
        .route("/api/tamper", post(routes::tamper))
        .route("/api/confirm", get(routes::confirm))
        .route("/api/market", get(routes::market))
        .layer(cors)
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(SocketAddr::from((host, port))).await?;
    println!(
        "Preflight API ready on http://{}.{}.{}.{}:{port}",
        host[0], host[1], host[2], host[3]
    );
    axum::serve(listener, app).await?;
    Ok(())
}
