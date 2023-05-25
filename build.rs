use std::process::Command;

fn main() {
    Command::new("tailwind")
        .args([
            "-i",
            "templates/main.css",
            "-o",
            "static/main.css",
            "--minify",
        ])
        .output()
        .expect("could not run tailwind, is it installed?");
}
