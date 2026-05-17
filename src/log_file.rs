use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

static LOG_PATH: OnceLock<PathBuf> = OnceLock::new();

pub fn init() {
    let path = log_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(&path, "");
    write("log initialized");
    install_panic_hook();
}

pub fn path() -> Option<&'static PathBuf> {
    LOG_PATH.get()
}

pub fn write(message: impl AsRef<str>) {
    let path = log_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "[{}] {}", timestamp_ms(), message.as_ref());
    }
}

fn log_path() -> &'static PathBuf {
    LOG_PATH.get_or_init(|| {
        #[cfg(target_os = "android")]
        {
            let public = PathBuf::from("/storage/emulated/0/stepmania/stepmania_r.log");
            if fs::create_dir_all(public.parent().unwrap()).is_ok() {
                return public;
            }
            let sdcard = PathBuf::from("/sdcard/stepmania/stepmania_r.log");
            if fs::create_dir_all(sdcard.parent().unwrap()).is_ok() {
                return sdcard;
            }
            PathBuf::from("stepmania_r.log")
        }
        #[cfg(not(target_os = "android"))]
        {
            PathBuf::from("stepmania_r.log")
        }
    })
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
