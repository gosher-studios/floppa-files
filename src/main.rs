#![feature(map_try_insert)]
mod config;
mod web;
//todo
//todo
//todo
//todo
//todo
//todo
//todo
//todo
//todo
//todo
//todo
use std::env;
use std::collections::HashMap;
use std::fmt::Debug;
use std::net::SocketAddr;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use std::path::PathBuf;
use std::fs::OpenOptions;
use std::time::Duration;
use axum_client_ip::InsecureClientIp;
use futures::lock::Mutex;
use nanoid::nanoid;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncSeekExt, AsyncWriteExt, BufWriter};
use tokio::{fs, io};
use tokio::net::TcpListener;
use tokio::fs::File;
use tokio::sync::{RwLock, mpsc};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use futures::TryStreamExt;
use axum::{Json, Router};
use axum::body::{Body, Bytes};
use axum::extract::{DefaultBodyLimit, Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Response, IntoResponse};
use axum::routing::{get, put};
use tokio_util::io::StreamReader;
use tower_http::services::ServeDir;
use tracing::{info, warn, error};
use tracing_subscriber::prelude::*;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::fmt::time::ChronoUtc;
use blake3::{hash, Hash, Hasher};
use timedmap::TimedMap;
use crate::config::Config;

type ArcState = Arc<AppState>;

const REPLACE_CHARS: [char; 13] = [
  '/', '\\', '&', '?', '"', '\'', '*', '~', '|', ':', '<', '>', ' ',
];
const VER: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug)]
struct AppState {
  file_count: RwLock<usize>,
  temp_files: RwLock<TimedMap<String, FileUpload>>,
  config: Config,
  path_tx: UnboundedSender<PathBuf>,
}

#[derive(Debug, Clone)]
struct FileUpload {
  file: Arc<Mutex<BufWriter<File>>>,
  file_name: String,
  chunk_size: usize,
  last_chunk: usize,
  last_hash: Hash,
}

#[tokio::main]
async fn main() -> Result<(), io::Error> {
  let config = Config::load(env::var("FLOPPA_CONFIG").unwrap_or("config.toml".into())).await?;

  let timer = ChronoUtc::new("%FT%T".into());
  tracing_subscriber::registry()
    .with(LevelFilter::INFO)
    .with(
      tracing_subscriber::fmt::layer()
        .with_writer(
          OpenOptions::new()
            .create(true)
            .append(true)
            .open(&config.log_file)?,
        )
        .with_ansi(false)
        .with_target(false)
        .with_timer(timer.clone()),
    )
    .with(tracing_subscriber::fmt::layer().with_timer(timer))
    .init();

  info!("floppa files v{}", VER);
  info!(
    max_size = config.max_size,
    file_dir = ?config.file_dir,
    allow_empty_files = config.allow_empty_files,
    prefix_length = config.prefix_length,
    compress_size = config.compress_count
  );

  if fs::create_dir(&config.file_dir).await.is_ok() {
    info!("created files directory {:?}", &config.file_dir)
  };

  let (path_tx, path_rx) = mpsc::unbounded_channel();
  let state = Arc::new(AppState {
    file_count: RwLock::new(std::fs::read_dir(&config.file_dir)?.count()),
    config: config.clone(),
    path_tx,
    temp_files: RwLock::new(TimedMap::new()),
  });
  tokio::spawn(deduper(config.clone(), path_rx));

  let app = Router::new()
    .route("/", get(web::home))
    .route("/tos", get(web::tos))
    .route(
      "/:id",
      put(upload).get_service(ServeDir::new(&config.file_dir)),
    )
    .route("/begin/:id/:size", get(begin_upload))
    .route("/end/:id", get(end_upload))
    .route("/up/:id/:idx", put(upload_new))
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
  InsecureClientIp(ip): InsecureClientIp,
  headers: HeaderMap,
  body: Body,
) -> Result<Response, AppError> {
  if !state.config.allow_empty_files {
    match headers
      .get("content-length")
      .and_then(|v| v.to_str().ok())
      .and_then(|v| v.parse::<u64>().ok())
    {
      Some(size) if size > 0 => {}
      _ => return Ok(StatusCode::BAD_REQUEST.into_response()),
    };
  }

  let file_name = format!(
    "{}.{}",
    nanoid::format(
      nanoid::rngs::default,
      &nanoid::alphabet::SAFE,
      state.config.prefix_length
    ),
    id.replace(REPLACE_CHARS, "-")
  );
  let path = state.config.file_dir.join(&file_name);
  let mut file = File::create(&path).await?;

  let mut reader = StreamReader::new(body.into_data_stream().map_err(io::Error::other));
  match io::copy(&mut reader, &mut file).await {
    Ok(_) => {
      info!("{} uploaded {}", ip, file_name);
      *state.file_count.write().await += 1;
      let _ = state.path_tx.send(path);
      Ok(file_name.into_response())
    }
    Err(err) => {
      warn!("error uploading {} ({})", file_name, err);
      fs::remove_file(&path).await?;
      Err(err.into())
    }
  }
}

#[derive(Serialize, Deserialize, Debug)]
struct Return {
  id: String,
  size: usize,
  count: usize,
}

#[axum::debug_handler]
async fn begin_upload(
  Path((id, size)): Path<(String, usize)>,
  State(state): State<ArcState>,
) -> Result<Response, AppError> {
  let c = &state.config;
  if size > state.config.max_size {
    return Ok((StatusCode::PAYLOAD_TOO_LARGE).into_response());
  }
  let file_name = format!(
    "{}.{}",
    nanoid::format(
      nanoid::rngs::default,
      &nanoid::alphabet::SAFE,
      c.prefix_length
    ),
    id.replace(REPLACE_CHARS, "-")
  );

  let ret_id = nanoid::format(nanoid::rngs::default, &nanoid::alphabet::SAFE, 16);
  let path = c.file_dir.join(&file_name);
  let file = File::create(&path).await?;
  // TODO replace magic number 32 with config
  let chunk_size = c.max_chunk.min(size / c.chunking_target);
  let buf = BufWriter::with_capacity(chunk_size * 32, file);
  let up: FileUpload = FileUpload {
    file: Arc::new(Mutex::new(buf)),
    last_chunk: 0,
    last_hash: Hash::from_bytes([0; 32]),
    file_name,
    chunk_size: chunk_size.clone(),
  };

  state.temp_files.write().await.deref().insert(
    ret_id.clone(),
    up,
    Duration::from_secs(c.delete_time as u64),
  );
  let r = Return {
    id: ret_id,
    count: size / chunk_size,
    size: chunk_size,
  };
  info!("created file for upload {:?}", r);
  Ok(Json(r).into_response())
}

// currently only writes to first data which is a bit confusing
async fn upload_new(
  Path((id, idx)): Path<(String, usize)>,
  State(state): State<ArcState>,
  headers: HeaderMap,
  body: Bytes,
) -> Result<Response, AppError> {
  let mut f = state.clone().temp_files.write().await.get(&id).unwrap();
  if !state.config.allow_empty_files {
    match headers
      .get("content-length")
      .and_then(|v| v.to_str().ok())
      .and_then(|v| v.parse::<u64>().ok())
    {
      Some(size) if size > 0 && size <= f.chunk_size as u64 => {}
      _ => return Ok(StatusCode::BAD_REQUEST.into_response()),
    };
  }
  let mut buf_writer = f.file.lock().await;
  let h = hash(&body.split_at(32).0);
  let seek = (idx % 32) * (f.chunk_size);
  //TODO fix seek so it can handle "concurrent"/"out of order operations"
  // let p = buf_writer
  //   .seek(std::io::SeekFrom::Start(seek as u64))
  //   .await
  //   .unwrap();

  let d = buf_writer.write(&body).await.unwrap();
  // todo log
  f.last_chunk += 1;
  f.last_hash = h;

  Ok(().into_response())
}

async fn end_upload(
  Path(id): Path<String>,
  State(state): State<ArcState>,
) -> Result<Response, AppError> {
  let f = state.clone().temp_files.write().await.get(&id).unwrap();
  let mut buf_writer = f.file.lock().await;
  buf_writer.flush().await.unwrap();
  let r = (f.file_name.clone()).into_response();
  let path = state.config.file_dir.join(&f.file_name);
  state.clone().temp_files.write().await.remove(&id).unwrap();
  info!("finished uploading file {:?}", r);
  *state.file_count.write().await += 1;
  let _ = state.path_tx.send(path);
  Ok(r)
}

async fn deduper(config: Config, mut rx: UnboundedReceiver<PathBuf>) -> Result<(), io::Error> {
  let mut hashes = HashMap::new();
  for entry in std::fs::read_dir(config.file_dir)? {
    let entry = entry?;
    if !entry.metadata()?.is_symlink() {
      check_dupes(&mut hashes, entry.path()).await?;
    }
  }
  while let Some(path) = rx.recv().await {
    check_dupes(&mut hashes, path).await?;
  }
  Ok(())
}

async fn check_dupes(hashes: &mut HashMap<Hash, String>, path: PathBuf) -> Result<(), io::Error> {
  let mut hasher = Hasher::new();
  hasher.update_mmap(&path)?;
  let hash = hasher.finalize();

  let file_name = path.file_name().unwrap().to_str().unwrap().to_string();
  if let Err(e) = hashes.try_insert(hash, file_name.clone()) {
    let original_path = e.entry.get();
    info!("deduplicating {:?}, copy of {:?}", file_name, original_path);
    fs::remove_file(&path).await?;
    fs::symlink(original_path, &path).await?;
  }
  Ok(())
}

#[derive(Debug)]
struct AppError(io::Error);

impl IntoResponse for AppError {
  fn into_response(self) -> Response {
    error!("{}", self.0);
    StatusCode::INTERNAL_SERVER_ERROR.into_response()
  }
}
impl From<io::Error> for AppError {
  fn from(value: io::Error) -> Self {
    Self(value)
  }
}
