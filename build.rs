use std::fs;
use std::process::Command;

const TAILWIND_VERSION: &str = "3.4.1";

fn main() {
  Command::new(if cfg!(windows) { "cmd" } else { "sh" })
    .args([
      if cfg!(windows) { "/C" } else { "-c" },
      format!(
        "npx -- tailwindcss@{} -i templates/main.css -o static/main.css --minify",
        TAILWIND_VERSION
      )
      .as_str(),
    ])
    .output()
    .expect("failed to run tailwind, is npm installed??????");
  fs::copy("qrcodejs/qrcode.min.js", "static/qrcode.min.js")
    .expect("could not copy qrcodejs, did you clone submodules????");
}
