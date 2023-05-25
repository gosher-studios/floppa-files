use askama::Template;
use async_std::{fs::OpenOptions, io};
use nanoid::nanoid;
use qrcode::render::svg;
use qrcode::QrCode;
use std::path::PathBuf;
use tide::{Body, Request, Response, StatusCode};

pub type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Clone)]
pub struct State {
    path: PathBuf,
}

#[async_std::main]
async fn main() -> Result {
    //figure out settings tomorrow
    tide::log::start();
    let mut app = tide::with_state(State {
        path: PathBuf::from("/home/gsh/proj/floppa-files/files"),
    });
    app.at("/").get(home);
    app.at("/qr/:url").get(qr);
    app.at("/:id").get(serve).put(upload);
    app.at("/static").serve_dir("static")?;
    app.listen("127.0.0.1:8080").await?;
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

pub async fn upload(mut req: Request<State>) -> tide::Result {
    let path = req.param("id")?;
    let mut fs_path: PathBuf = req.state().clone().path;
    let fpath = nanoid!(8) + "." + &str::replace(path, "%20", "-");
    fs_path.push(&fpath);
    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .open(&fs_path)
        .await?;
    let bytes_written = io::copy(req, file).await?;
    let f: String = "https://colon3.lol/".to_string() + &fpath;
    let res = tide::Response::builder(200).body(f).build();
    Ok(res)
}

pub async fn qr(req: Request<State>) -> tide::Result {
    let url = req.param("url")?;
    let f = "https://colon3.lol/".to_string() + &url;
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
    total: i8,
    max: i8,
}

pub async fn home(req: Request<State>) -> tide::Result {
    let home: Home = Home {
        url: "colon3.lol".to_string(),
        total: 4,
        max: 69,
    };
    let mut body = Body::from_string(home.render()?);
    body.set_mime(Home::MIME_TYPE);
    Ok(body.into())
}
