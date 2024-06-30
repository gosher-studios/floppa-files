use std::net::SocketAddr;
use std::path::{PathBuf, Path};
use tokio::{fs, io};
use serde::{Serialize, Deserialize};
use crate::Result;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct Config {
  pub max_size: usize,
  pub file_dir: PathBuf,
  pub listen: SocketAddr,
  pub log_file: PathBuf,
}

impl Default for Config {
  fn default() -> Self {
    Self {
      max_size: 5 * 1000 * 1000 * 1000,
      file_dir: "files".into(),
      listen: ([0, 0, 0, 0], 3000).into(),
      log_file: format!("{}.log", env!("CARGO_BIN_NAME")).into(),
    }
  }
}

impl Config {
  pub async fn load<P: AsRef<Path>>(file: P) -> Result<Self> {
    let config = match fs::read_to_string(&file).await {
      Ok(contents) => toml::from_str(&contents)?,
      Err(err) => match err.kind() {
        io::ErrorKind::NotFound => {
          let default_config = Config::default();
          fs::write(&file, toml::to_string_pretty(&default_config)?).await?;
          default_config
        }
        _ => return Err(err.into()),
      },
    };
    Ok(config)
  }
}
