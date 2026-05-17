use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const LOG_NAME: &str = "stepmania_r.log";

pub fn init() {
    for path in log_paths() {
        prepare_log_file(&path, true);
    }
    write("log initialized");
    install_panic_hook();
}

pub fn paths() -> Vec<PathBuf> {
    log_paths()
}

pub fn write(message: impl AsRef<str>) {
    let line = format!("[{}] {}", timestamp_ms(), message.as_ref());
    for path in log_paths() {
        prepare_log_file(&path, false);
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
            let _ = writeln!(file, "{line}");
        }
    }
}

fn log_paths() -> Vec<PathBuf> {
    #[cfg(target_os = "android")]
    {
        vec![
            PathBuf::from("/sdcard").join(LOG_NAME),
            PathBuf::from("/sdcard/Download").join(LOG_NAME),
            PathBuf::from("/sdcard/Documents").join(LOG_NAME),
            PathBuf::from("/sdcard/stepmania").join(LOG_NAME),
            PathBuf::from("/storage/emulated/0").join(LOG_NAME),
            PathBuf::from("/storage/emulated/0/Download").join(LOG_NAME),
            PathBuf::from("/storage/emulated/0/Documents").join(LOG_NAME),
            PathBuf::from("/storage/emulated/0/stepmania").join(LOG_NAME),
            PathBuf::from(LOG_NAME),
        ]
    }
    #[cfg(not(target_os = "android"))]
    {
        vec![PathBuf::from(LOG_NAME)]
    }
}

fn prepare_log_file(path: &Path, truncate: bool) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if truncate {
        let _ = fs::write(path, "");
    }
}

fn timestamp_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

fn install_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        write(format!("panic: {info}"));
    }));
}
