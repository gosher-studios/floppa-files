mod config;

use std::net::{SocketAddrV4, Ipv4Addr};
use std::error::Error;
use std::fs::{self, File, Permissions};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::io::Write;
use std::process::Command;
use std::collections::BTreeMap;
use simplelog::{TermLogger, LevelFilter, Config as LogConfig, TerminalMode, ColorChoice};
use log::info;
use warp::{Filter, Rejection};
use bytes::Bytes;
use handlebars::Handlebars;
use rand::Rng;
use humansize::{format_size, DECIMAL};
use crate::config::CONFIG;

pub type Result<T = (), E = Box<dyn Error>> = std::result::Result<T, E>;
const TAILWIND_URL: &str =
  "https://github.com/tailwindlabs/tailwindcss/releases/download/v3.1.8/tailwindcss-linux-x64";

#[tokio::main]
async fn main() -> Result<()> {
  TermLogger::init(
    LevelFilter::Info,
    LogConfig::default(),
    TerminalMode::Stdout,
    ColorChoice::Auto,
  )?;
  info!("{} v{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
  fs::create_dir_all(CONFIG.files_dir)?;

  let mut hb = Handlebars::new();
  let mut data = BTreeMap::new();
  data.insert("max_size", format_size(CONFIG.max_size, DECIMAL));
  data.insert("base_url", CONFIG.base_url.to_string());
  data.insert("version", env!("CARGO_PKG_VERSION").to_string());
  hb.register_template_string("index", include_str!("index.hbs"))?;

  if !Path::new("tailwind").exists() {
    info!("Downloading Tailwind from {}.", TAILWIND_URL);
    let mut tw = File::create("tailwind")?;
    tw.set_permissions(Permissions::from_mode(0o755))?;
    tw.write_all(&reqwest::get(TAILWIND_URL).await?.bytes().await?)?;
    drop(tw);
  }

  let out = Command::new("./tailwind")
    .args(["-i", "src/base.css", "-o", "static/tw.css", "--minify"])
    .output()?;
  info!(
    "Tailwind {}",
    String::from_utf8(out.stderr)?
      .trim()
      .replace("Done", "compiled")
  );

  let static_files = warp::fs::dir("static");
  let page = warp::path::end().map(move || {
    let mut data = data.clone();
    data.insert("img", rand::thread_rng().gen_range(1..=4).to_string());
    let (mut files, mut size) = (0, 0);
    for file in fs::read_dir(CONFIG.files_dir).unwrap() {
      files += 1;
      size += file.unwrap().metadata().unwrap().len();
    }
    data.insert("files", files.to_string());
    data.insert("size", format_size(size, DECIMAL));
    warp::reply::html(hb.render("index", &data).unwrap())
  });
  let down = warp::fs::dir(CONFIG.files_dir);
  let up = warp::path("up")
    .and(warp::path::param())
    .and(warp::body::content_length_limit(CONFIG.max_size))
    .and(warp::body::bytes())
    .and_then(upload);

  let routes = warp::get()
    .and(static_files.or(page).or(down))
    .or(warp::post().and(up))
    .with(warp::log("info"));

  warp::serve(routes)
    .run(SocketAddrV4::new(Ipv4Addr::LOCALHOST, CONFIG.port))
    .await;
  Ok(())
}

async fn upload(file: String, data: Bytes) -> Result<String, Rejection> {
  if data.is_empty() {
    Ok("no file attached".to_string())
  } else {
    let filename = format!("{}.{}", nanoid::nanoid!(10), file);
    info!("Creating file {}.", filename);
    File::create(format!("files/{}", filename))
      .unwrap()
      .write_all(&data)
      .unwrap();
    Ok(format!("{}/{}\n", CONFIG.base_url, filename))
  }
}
