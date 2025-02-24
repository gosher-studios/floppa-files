use askama::Template;
use axum::extract::State;
use crate::{ArcState, VER};

#[derive(Template)]
#[template(path = "home.html")]
pub struct Home {
  total: usize,
  max: usize,
  allow_empty: bool,
  compress_count: usize,
  ver: &'static str,
}

pub async fn home(State(state): State<ArcState>) -> Home {
  Home {
    total: *state.file_count.read().await,
    max: state.config.max_size,
    allow_empty: state.config.allow_empty_files,
    compress_count: state.config.compress_count,
    ver: VER,
  }
}

#[derive(Template)]
#[template(path = "tos.html")]
pub struct Tos {
  ver: &'static str,
}

pub async fn tos() -> Tos {
  Tos { ver: VER }
}
