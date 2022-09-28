use std::fs::{self, File};
use std::path::Path;
use std::io::Write;
use std::process;
use serde::{Serialize, Deserialize};
use lazy_static::lazy_static;
use log::{info, error};
use humansize::{format_size, DECIMAL};
use crate::Result;

#[derive(Serialize, Deserialize)]
pub struct Config {
  pub port: u16,
  pub max_size: u64,
  pub files_dir: &'static str,
  pub base_url: &'static str,
}

impl Default for Config {
  fn default() -> Self {
    Self {
      port: 4040,
      max_size: 1000000000,
      files_dir: "files",
      base_url: "http://localhost:4040",
    }
  }
}

impl Config {
  fn load(path: &str) -> Result<Self> {
    if !Path::new(path).exists() {
      info!("Creating config {}.", path);
      File::create(path)?.write(&toml::to_string_pretty(&Self::default())?.as_bytes())?;
    }
    match toml::from_str::<Self>(Box::leak(fs::read_to_string(path)?.into_boxed_str())) {
      Ok(c) => {
        info!("Loaded config from {}.", path);
        info!(" port {}", c.port);
        info!(" max_size {}", format_size(c.max_size,DECIMAL));
        info!(" files_dir {}", c.files_dir);
        info!(" base_url {}", c.base_url);
        Ok(c)
      }
      Err(e) => {
        error!("Couldnt parse config.\n{}", e);
        process::exit(1);
      }
    }
  }
}

lazy_static! {
  pub static ref CONFIG: Config = Config::load("floppa_files.toml").unwrap();
}
