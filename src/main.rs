use askama::Template;
use axum::{
    body::StreamBody,
    extract::{BodyStream, ConnectInfo, DefaultBodyLimit, Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, put},
    Router,
};
use futures::StreamExt;
use nanoid::nanoid;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, net::SocketAddr, path::PathBuf, sync::Arc};
use tokio::{
    fs::File,
    io::{AsyncWriteExt, BufWriter},
    signal,
};
use tokio::{io::AsyncReadExt, sync::RwLock};
use tokio_util::io::ReaderStream;
use tower_http::services::ServeDir;

#[derive(Deserialize, Clone, Debug, Serialize)]
struct AppState {
    max_size: usize,
    url: String,
    file_dir: PathBuf,
    listen: SocketAddr,
    #[serde(skip)]
    file_count: Arc<RwLock<usize>>,
    #[serde(skip)]
    logs: Arc<RwLock<HashMap<String, Vec<String>>>>,
}

#[tokio::main]
async fn main() {
    let mut state: AppState =
        toml::from_str(&tokio::fs::read_to_string("config.toml").await.unwrap()).unwrap();
    if tokio::fs::File::open("logfile").await.is_err() {
        let mut empty = tokio::fs::File::create("logfile").await.unwrap();
        let mut hash: HashMap<String, Vec<String>> = HashMap::new();
        let vecr: Vec<String> = vec!["meow.sh".to_string()];
        hash.insert("0.0.0.0".to_string(), vecr);
        let serialized = bincode::serialize(&hash).unwrap();
        empty.write_all(&serialized).await.unwrap();
        empty.shutdown().await.unwrap();
        println!("created empty log file");
    }
    let mut file = tokio::fs::File::open("logfile").await.unwrap();
    let mut output = vec![];
    file.read_to_end(&mut output).await.unwrap();
    let load: HashMap<String, Vec<String>> = bincode::deserialize(&output).unwrap();
    state.logs = Arc::new(RwLock::new(load));
    if tokio::fs::create_dir(&state.file_dir).await.is_ok() {
        println!("created files directory in {:?}", &state.file_dir)
    };
    state.file_count = Arc::new(RwLock::new(
        std::fs::read_dir(&state.file_dir).unwrap().count(),
    ));
    let addr = state.listen;
    let app = Router::new()
        .route("/", get(home))
        .route("/:id", put(upload).get(download))
        .route("/qr/:id", get(qr))
        .layer(DefaultBodyLimit::max(state.max_size))
        .nest_service("/static", ServeDir::new("static"))
        .with_state(state.clone().to_owned());
    println!("app initialized");
    axum::Server::bind(&addr)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .with_graceful_shutdown(async {
            signal::ctrl_c().await.unwrap();
            println!("shutting down");
            let mut file = File::create("logfile").await.unwrap();
            let writer = bincode::serialize(&*state.logs.read_owned().await);
            file.write_all(&writer.unwrap()).await.unwrap();
        })
        .await
        .unwrap();
}

async fn upload(
    Path(id): Path<String>,
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    mut stream: BodyStream,
) -> Response {
    let res: Arc<String> = Arc::new(
        nanoid!(8)
            + "."
            + &id
                .replace(
                    [
                        '/', '\\', '&', '?', '"', '\'', '*', '~', '|', ':', '<', '>', ' ',
                    ],
                    "-",
                )
                .replace("%20", "-"),
    );
    let path = state.file_dir.join(Arc::clone(&res).to_string());
    let file = tokio::fs::File::create(&path).await.unwrap();
    let mut bug = BufWriter::new(file);
    while let Some(chunk) = stream.next().await {
        if bug.write_all(chunk.unwrap().as_ref()).await.is_err() {
            bug.flush().await.unwrap();
            tokio::fs::remove_file(path.clone()).await.unwrap();
            break;
        }
    }
    state
        .logs
        .write()
        .await
        .entry(addr.ip().to_string())
        .or_insert(vec![Arc::clone(&res).to_string()])
        .push(Arc::clone(&res).to_string());
    *state.file_count.write().await += 1;

    (StatusCode::OK, res.to_string()).into_response()
}

async fn download(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    let meow = state.file_dir.join(id);
    let file = match File::open(&meow).await {
        Ok(file) => file,
        Err(_) => return (StatusCode::NOT_FOUND).into_response(),
    };
    let stream = ReaderStream::new(file);
    let body = StreamBody::new(stream);
    (StatusCode::OK, body).into_response()
}

#[derive(Template)]
#[template(path = "home.html")]
struct Home {
    url: String,
    total: usize,
    max: String,
    ver: String,
}
async fn home(State(state): State<AppState>) -> Home {
    Home {
        url: state.url,
        total: *state.file_count.read().await,
        max: format_size(state.max_size.to_string()),
        ver: "4".to_string(),
    }
}

fn format_size(size: String) -> String {
    let sizes = ["B", "KB", "MB", "GB", "TB", "PB"];
    let mut take = size.len() % 3;
    let mut modifier = 0;
    if take == 0 {
        take = 3;
        modifier = 1;
    }
    format!(
        "{}{}",
        size.chars().take(take).collect::<String>(),
        sizes[size.len() / 3 - modifier]
    )
}

async fn qr(Path(id): Path<String>, State(state): State<AppState>) -> Response {
    let url = state.url + &id;
    let qr = qrcode_generator::to_svg_to_string(
        url,
        qrcode_generator::QrCodeEcc::Low,
        200,
        None::<&str>,
    )
    .unwrap();
    (StatusCode::OK, qr).into_response()
}
