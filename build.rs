use std::process::Command;

fn main() {
    if cfg!(target_os = "windows") {
        Command::new("cmd")
            .args([
                "/C",
                "tailwind -i templates/main.css -o static/main.css --minify",
            ])
            .output()
            .expect("failed to run tailwind, is tailwind installed????")
    } else if cfg!(target_os = "macos") {
        Command::new("sh")
            .args([
                "-c",
                "tailwindcss -i templates/main.css -o static/main.css --minify",
            ])
            .output()
            .expect("failed to run tailwind, is tailwind installed????")
    } else {
        Command::new("tailwind")
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
