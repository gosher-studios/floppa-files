#![feature(map_try_insert)]
mod config;
mod log;

use std::collections::HashMap;
use std::env;
use std::fmt::Debug;
use std::net::SocketAddr;
use std::sync::Arc;
use axum_client_ip::InsecureClientIp;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::{fs, io};
use tokio::sync::{mpsc, RwLock};
use tokio::net::TcpListener;
use tokio::fs::{read_dir, remove_file, symlink, File};
use futures::TryStreamExt;
use axum::Router;
use axum::body::Body;
use axum::extract::{DefaultBodyLimit, Path, State};
use axum::http::StatusCode;
use axum::response::{Response, IntoResponse};
use axum::routing::{get, put};
use tokio_stream::wrappers::ReadDirStream;
use tokio_stream::StreamExt;
use tower_http::services::ServeDir;
use askama::Template;
use nanoid::nanoid;
use tracing::{info, debug, error};
use tracing_subscriber::filter::EnvFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use blake3::{Hash, Hasher};
use config::Config;
use log::Logger;

pub type Result<T = (), E = Box<dyn std::error::Error>> = std::result::Result<T, E>;

#[derive(Debug)]
struct AppState {
  file_count: RwLock<usize>,
  config: Config,
  hashes: RwLock<HashMap<Hash, String>>,
  logger: Logger,
  tx: UnboundedSender<String>,
}

type ArcState = Arc<AppState>;

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
  let (tx, rx) = mpsc::unbounded_channel();
  let config = Config::load(env::var("FLOPPA_CONFIG").unwrap_or("config.toml".into())).await?;
  let logger = Logger::new(&config.log_file);

  if fs::create_dir(&config.file_dir).await.is_ok() {
    info!("created files directory {:?}", &config.file_dir)
  };

  let state = Arc::new(AppState {
    file_count: RwLock::new(std::fs::read_dir(&config.file_dir)?.count()),
    config: config.clone(),
    logger,
    hashes: RwLock::new(HashMap::new()),
    tx,
  });
  let cloned_state = state.clone();
  let _manager = tokio::spawn(async move { manager(cloned_state, rx).await });
  let mut stream = ReadDirStream::new(read_dir(&config.file_dir).await?);
  info!("meowing the files directory");
  while let Some(file) = stream.next().await {
    let f = file?;
    if !&f.metadata().await?.is_symlink() {
      check_hash(state.clone(), f.file_name().into_string().unwrap())
        .await
        .unwrap();
    }
  }
  let app = Router::new()
    .route("/", get(home))
    .route("/tos", get(tos))
    .route(
      "/:id",
      put(upload).get_service(ServeDir::new(&config.file_dir)),
    )
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

async fn manager(state: ArcState, mut rx: UnboundedReceiver<String>) -> Result<(), AppError> {
  while let Some(name) = rx.recv().await {
    check_hash(state.clone(), name).await?;
  }
  Ok(())
}

async fn upload(
  Path(id): Path<String>,
  State(state): State<ArcState>,
  InsecureClientIp(ip): InsecureClientIp,
  body: Body,
) -> Result<Response, AppError> {
  let file_name = id.replace(
    [
      '/', '\\', '&', '?', '"', '\'', '*', '~', '|', ':', '<', '>', ' ',
    ],
    "-",
  );
  let file_name = format!("{}.{}", nanoid!(8), file_name);
  let path = state.config.file_dir.join(&file_name);
  let mut file = File::create(&path).await.unwrap();
  let mut reader = tokio_util::io::StreamReader::new(
    body
      .into_data_stream()
      .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err)),
  );
  if let Err(error) = io::copy(&mut reader, &mut file).await {
    debug!("removing failed upload {}", &file_name);
    remove_file(&path).await?;
    Err(error.into())
  } else {
    state
      .logger
      .log(format!("{} uploaded {}", &ip, &file_name))?;
    *state.file_count.write().await += 1;
    state.tx.clone().send(file_name.clone()).unwrap();
    Ok(file_name.into_response())
  }
}

async fn check_hash(state: ArcState, file_name: String) -> Result<(), AppError> {
  let mut hasher = Hasher::new();
  let path = state.config.file_dir.join(&file_name);
  hasher.update_mmap(&path)?;
  let hash = hasher.finalize();
  let mut hashes = state.hashes.write().await;
  match hashes.try_insert(hash, file_name) {
    Ok(_) => {}
    Err(e) => {
      let vpath = state.config.file_dir.join(&e.entry.get());
      state.logger.log(format!(
        "file {:?} deduplicated, symlinked to {:?}",
        &path.file_name().unwrap(),
        &vpath.file_name().unwrap()
      ))?;
      remove_file(&path).await?;
      symlink(vpath, path).await?;
    }
  };

  Ok(())
}

#[derive(Template)]
#[template(path = "home.html")]
struct Home {
  total: usize,
  max: usize,
  ver: &'static str,
}

async fn home(State(state): State<ArcState>) -> Home {
  Home {
    total: *state.file_count.read().await,
    max: state.config.max_size,
    ver: env!("CARGO_PKG_VERSION"),
  }
}

#[derive(Template)]
#[template(path = "tos.html")]
struct Tos {
  ver: &'static str,
}

async fn tos() -> Tos {
  Tos {
    ver: env!("CARGO_PKG_VERSION"),
  }
}

#[derive(Debug)]
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
