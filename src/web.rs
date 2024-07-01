use askama::Template;
use axum::extract::State;
use crate::ArcState;

#[derive(Template)]
#[template(path = "home.html")]
pub struct Home {
  total: usize,
  max: usize,
  ver: &'static str,
}

pub async fn home(State(state): State<ArcState>) -> Home {
  Home {
    total: *state.file_count.read().await,
    max: state.config.max_size,
    ver: env!("CARGO_PKG_VERSION"),
  }
}

#[derive(Template)]
#[template(path = "tos.html")]
pub struct Tos {
  ver: &'static str,
}

pub async fn tos() -> Tos {
  Tos {
    ver: env!("CARGO_PKG_VERSION"),
  }
}
