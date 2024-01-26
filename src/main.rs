mod config;
mod log;

use askama::Template;
use axum::{
    body::Body,
    extract::{ConnectInfo, DefaultBodyLimit, Path, State},
    http::{StatusCode, HeaderMap},
    response::{IntoResponse, Response},
    routing::{get, put},
    Router,
};
use futures::TryStreamExt;
use config::Config;
use log::Logger;
use nanoid::nanoid;
use tracing::{info, debug, error};
use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use tokio::{
    fs::File,
    io::{AsyncWriteExt, BufWriter},
    signal,
    net::TcpListener,
};
use tokio::{io::AsyncReadExt, sync::RwLock};
use tower_http::services::ServeDir;
use tracing_subscriber::{filter::EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};
use humansize::{format_size, DECIMAL};

pub type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Debug)]
struct AppState {
    file_count: RwLock<usize>,
    config: Config,
    logger: Logger,
}

type ArcState = Arc<AppState>;

struct AppError(std::io::Error);

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        error!("{}", self.0);
        StatusCode::INTERNAL_SERVER_ERROR.into_response()
    }
}

impl From<std::io::Error> for AppError {
    fn from(value: std::io::Error) -> Self {
        Self(value)
    }
}

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() -> Result {
    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env()
                .or_else(|_| EnvFilter::try_new("info"))
                .unwrap(),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config =
        config::load(std::env::var("FLOPPA_CONFIG").unwrap_or("config.toml".into())).await?;

    let logger = Logger::new(&config.log_file);

    if tokio::fs::create_dir(&config.file_dir).await.is_ok() {
        info!("created files directory in {:?}", &config.file_dir)
    };

    let state = Arc::new(AppState {
        file_count: RwLock::new(std::fs::read_dir(&config.file_dir)?.count()),
        config: config.clone(),
        logger,
    });

    let serve_files = ServeDir::new(&config.file_dir);

    let app = Router::new()
        .route("/", get(home))
        .route("/:id", put(upload).get_service(serve_files))
        .layer(DefaultBodyLimit::max(config.max_size))
        .nest_service("/static", ServeDir::new("static"))
        .with_state(state);

    let listener = TcpListener::bind(&config.listen).await?;
    info!("server listening on http://{}", listener.local_addr()?);
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}

async fn upload(
    Path(id): Path<String>,
    State(state): State<ArcState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    body: Body,
) -> std::result::Result<Response, AppError> {
    let file_name = id.replace(
        [
            '/', '\\', '&', '?', '"', '\'', '*', '~', '|', ':', '<', '>', ' ',
        ],
        "-",
    );
    let file_name = format!("{}.{}", nanoid!(8), file_name);
    let path = state.config.file_dir.join(&file_name);
    let mut file = tokio::fs::File::create(&path).await.unwrap();

    let mut reader = tokio_util::io::StreamReader::new(
        body.into_data_stream()
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err)),
    );
    if let Err(error) = tokio::io::copy(&mut reader, &mut file).await {
        debug!("removing failed upload {}", &file_name);
        tokio::fs::remove_file(&path).await?;
        Err(error.into())
    } else {
        state
            .logger
            .log(format!("{} uploaded {}", &addr, &file_name))?;
        *state.file_count.write().await += 1;

        Ok(file_name.into_response())
    }
}

#[derive(Template)]
#[template(path = "home.html")]
struct Home {
    total: usize,
    max: String,
    ver: &'static str,
}

async fn home(State(state): State<ArcState>) -> Home {
    Home {
        total: *state.file_count.read().await,
        max: format_size(state.config.max_size, DECIMAL),
        ver: VERSION,
    }
}
