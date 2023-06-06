use askama::Template;
use async_std::stream::StreamExt;
use async_std::{fs::OpenOptions, io};
use nanoid::nanoid;
use qrcode::render::svg;
use qrcode::QrCode;
use serde::Deserialize;
use std::path::PathBuf;
use tide::http::headers::HeaderValue;
use tide::security::CorsMiddleware;
use tide::{Body, Request, Response, StatusCode};
pub type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Deserialize, Clone)]
pub struct State {
    path: PathBuf,
    url: String,
    max: u64,
    port: String,
}

#[async_std::main]
async fn main() -> Result {
    tide::log::start();
    let toml_content = async_std::fs::read_to_string("settings.toml")
        .await
        .unwrap();
    let state: State = toml::from_str(&toml_content)?;
    let port = state.port.clone();
    let mut app = tide::with_state(state);

    app.with(CorsMiddleware::new().allow_methods("GET, PUT".parse::<HeaderValue>()?));
    app.at("/").get(home);
    app.at("/qr/:url").get(qr);
    app.at("/:id").get(serve).put(upload);
    app.at("/static").serve_dir("static")?;
    app.listen(&port).await?;
    Ok(())
}

pub async fn serve(req: Request<State>) -> tide::Result {
    let file: String = req.param("id")?.into();
    let mut fs_path: PathBuf = req.state().clone().path;
    fs_path.push(file);
    if let Ok(body) = Body::from_file(fs_path).await {
        Ok(body.into())
    } else {
        Ok(Response::new(StatusCode::NotFound))
    }
}

pub async fn upload(req: Request<State>) -> tide::Result {
    println!("{:?}", req.len());
    match req.len() {
        Some(size) if !(size as u64 > req.state().clone().max) => {}
        _ => return Ok(Response::new(413)),
    };
    let path = req.param("id")?;
    let mut fs_path: PathBuf = req.state().clone().path;
    let fpath = nanoid!(8) + "." + &str::replace(path, "%20", "-");
    fs_path.push(&fpath);
    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .open(&fs_path)
        .await?;
    let _bytes_written = io::copy(req, file).await?;
    let f: String = fpath;
    let res = tide::Response::builder(200).body(f).build();
    Ok(res)
}

pub async fn qr(req: Request<State>) -> tide::Result {
    let url = req.param("url")?;
    let f = req.state().clone().url + &url;
    let qr = QrCode::new(f.as_bytes()).unwrap();
    let img = qr
        .render()
        .min_dimensions(100, 100)
        .dark_color(svg::Color("#ffffff"))
        .light_color(svg::Color("#000000"))
        .build();
    let res = tide::Response::builder(200).body(img).build();
    Ok(res)
}

#[derive(Template)]
#[template(path = "home.html")]
struct Home {
    url: String,
    total: usize,
    max: String,
}

pub async fn home(req: Request<State>) -> tide::Result {
    let r = req.state().clone();
    let home: Home = Home {
        url: r.url,
        total: async_std::fs::read_dir("files").await?.count().await,
        max: format_size(r.max.to_string()),
    };
    let mut body = Body::from_string(home.render()?);
    body.set_mime(Home::MIME_TYPE);
    Ok(body.into())
}
pub fn format_size(size: String) -> String {
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
