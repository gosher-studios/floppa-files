use async_std::{io, fs};
use async_std::fs::File;
use async_std::stream::StreamExt;
use std::net::SocketAddr;
use tide::{Body, Request, Response};
use tide::http::headers::HeaderValue;
use tide::security::CorsMiddleware;
use nanoid::nanoid;
use qrcode::QrCode;
use qrcode::render::svg::Color;
use askama::Template;
use serde::Deserialize;

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Deserialize, Clone)]
struct Config {
  listen: SocketAddr,
  file_dir: String,
  base_url: String,
  max_size: usize,
}

#[async_std::main]
async fn main() -> Result {
  tide::log::start();
  let conf: Config = toml::from_str(&fs::read_to_string("config.toml").await?)?;
  let mut app = tide::with_state(conf.clone());

  app.with(CorsMiddleware::new().allow_methods("GET, PUT".parse::<HeaderValue>()?));
  app.at("/").get(home).serve_dir("files")?;
  app.at("/qr/:url").get(qr);
  app.at("/:id").put(upload);
  app.at("/static").serve_dir("static")?;
  app.listen(conf.listen).await?;
  Ok(())
}

async fn upload(req: Request<Config>) -> tide::Result {
  let conf = req.state();
  if req.len().unwrap_or(usize::MAX) > conf.max_size {
    return Ok(Response::new(413));
  }
  let name = format!(
    "{}/{}.{}",
    conf.file_dir,
    nanoid!(8),
    req.param("id")?.replace("%20", "-")
  );
  io::copy(req, File::create(name.clone()).await?).await?;
  Ok(name.into())
}

async fn qr(req: Request<Config>) -> tide::Result {
  let conf = req.state().clone();
  let url = conf.base_url + req.param("url")?;
  let qr = QrCode::new(url.as_bytes())?
    .render()
    .min_dimensions(100, 100)
    .dark_color(Color("#fff"))
    .light_color(Color("#000"))
    .build();
  Ok(qr.into())
}

#[derive(Template)]
#[template(path = "home.html")]
struct Home {
  url: String,
  total: usize,
  max: String,
}

async fn home(req: Request<Config>) -> tide::Result {
  let conf = req.state().clone();
  let mut body = Body::from_string(
    Home {
      url: conf.base_url,
      total: fs::read_dir("files").await?.count().await,
      max: format_size(conf.max_size.to_string()),
    }
    .render()?,
  );
  body.set_mime(Home::MIME_TYPE);
  Ok(body.into())
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
