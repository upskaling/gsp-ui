use log::error;
use std::process::{Command, Stdio};

#[cfg(target_os = "linux")]
pub fn screenshot_region(path: &str) {
    let result = Command::new("xfce4-screenshooter")
        .arg("--region")
        .arg("--save")
        .arg(path)
        .stdout(Stdio::piped())
        .output();

    if let Err(e) = result {
        error!(
            "[SCREENSHOOTER] Erreur lors de la capture d'écran (xfce4-screenshooter): {}",
            e
        );
    }
}

#[cfg(target_os = "macos")]
pub fn screenshot_region(path: &str) {
    let result = Command::new("screencapture")
        .arg("-i")
        .arg("-r")
        .arg(path)
        .stdout(Stdio::piped())
        .output();

    if let Err(e) = result {
        error!(
            "[SCREENSHOOTER] Erreur lors de la capture d'écran (screencapture): {}",
            e
        );
    }
}
