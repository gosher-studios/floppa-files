use std::process::Command;

fn main() {
    if cfg!(target_os = "windows") {
        Command::new("cmd")
            .args([
                "/C",
                "tailwindcss -i templates/main.css -o static/main.css --minify",
            ])
            .output()
            .expect("failed to run tailwind, is tailwind installed????")
    } else {
        Command::new("tailwindcss")
            .args([
                "-i",
                "templates/main.css",
                "-o",
                "static/main.css",
                "--minify",
            ])
            .output()
            .expect("failed to run tailwind, is tailwind installed????")
    };
}
