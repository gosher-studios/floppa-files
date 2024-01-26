use std::fmt::Display;
use std::path::Path;
use std::fs::{File, OpenOptions};
use std::io::{Result, Write};
use std::sync::Mutex;

use chrono::SecondsFormat;

#[derive(Debug)]
pub struct Logger {
  file: Mutex<File>,
}

impl Logger {
  pub fn new<P: AsRef<Path>>(path: P) -> Self {
    Self {
      file: Mutex::new(
        OpenOptions::new()
          .append(true)
          .read(false)
          .create(true)
          .open(&path)
          .unwrap(),
      ),
    }
  }

  pub fn log<D: Display>(&self, thing: D) -> Result<()> {
    let now = chrono::Utc::now();
    let mut file = self.file.lock().unwrap();
    writeln!(
      file,
      "[{}]  {}",
      now.to_rfc3339_opts(SecondsFormat::Secs, true),
      thing
    )
  }
}
