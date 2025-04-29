use chrono::Local;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::panic::{self, PanicHookInfo};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Copy)]
enum TimestampPrecision {
    Second,
    Milli,
    Nano,
}

pub enum LogLevel {
    CronjobExecution,
    CronjobError,
    Trace,
    Debug,
    Info,
    Warning,
    Error,
}

impl LogLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::CronjobExecution => "CRONJOB",
            LogLevel::CronjobError => "CRONJOB_ERROR",
            LogLevel::Trace => "TRACE",
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warning => "WARNING",
            LogLevel::Error => "ERROR",
        }
    }

    fn timestamp_precision(&self) -> TimestampPrecision {
        match self {
            LogLevel::Trace => TimestampPrecision::Nano,
            LogLevel::Debug | LogLevel::CronjobExecution | LogLevel::CronjobError => TimestampPrecision::Milli,
            LogLevel::Info | LogLevel::Warning | LogLevel::Error => TimestampPrecision::Second,
        }
    }
}

pub struct CatalystLogger;

impl CatalystLogger {
    fn get_timestamp(level: &LogLevel) -> String {
        let precision = level.timestamp_precision();
        let now = SystemTime::now();
        let duration = now.duration_since(UNIX_EPOCH).unwrap_or_default();
        let local_time = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

        match precision {
            TimestampPrecision::Second => {
                format!("{}-{}", duration.as_secs(), local_time)
            }
            TimestampPrecision::Milli => {
                format!("{}.{:03}-{}", duration.as_secs(), duration.subsec_millis(), local_time)
            }
            TimestampPrecision::Nano => {
                format!("{}.{:09}-{}", duration.as_secs(), duration.subsec_nanos(), local_time)
            }
        }
    }

    pub fn log(level: LogLevel, message: &str) {
        let log_dir = "storage/logs";
        if let Err(e) = fs::create_dir_all(log_dir) {
            eprintln!("Failed to create log directory: {}", e);
            return;
        }
        let file_path = Path::new(log_dir).join(format!("{}.log", level.as_str().to_lowercase()));

        let timestamp = Self::get_timestamp(&level);
        let file_log_entry = format!("{} [{}] {}\n", timestamp, level.as_str(), message);

        if let Err(e) = OpenOptions::new().append(true).create(true).open(&file_path).and_then(|mut file| file.write_all(file_log_entry.as_bytes())) {
            eprintln!("Failed to write log: {}", e);
        }
    }

    pub fn log_to_path<P: AsRef<Path>>(path: P, level: LogLevel, message: &str) {
        let path = path.as_ref();
        if let Some(dir) = path.parent() {
            if let Err(e) = fs::create_dir_all(dir) {
                eprintln!("Failed to create directory {}: {}", dir.display(), e);
                return;
            }
        }

        let timestamp = Self::get_timestamp(&level);
        let file_log_entry = format!("{} [{}] {}\n", timestamp, level.as_str(), message);

        if let Err(e) = OpenOptions::new().append(true).create(true).open(path).and_then(|mut file| file.write_all(file_log_entry.as_bytes())) {
            eprintln!("Failed to write log to {}: {}", path.display(), e);
        }
    }
}

pub fn setup_panic_hook() {
    panic::set_hook(Box::new(|panic_info: &PanicHookInfo| {
        let payload = panic_info
            .payload()
            .downcast_ref::<&str>()
            .copied()
            .or_else(|| panic_info.payload().downcast_ref::<String>().map(|s| s.as_str()))
            .unwrap_or("Unknown panic");
        let location = if let Some(loc) = panic_info.location() {
            format!("{}:{}", loc.file(), loc.line())
        } else {
            "unknown location".to_string()
        };
        let full_msg = format!("Panic at {}: {}", location, payload);
        CatalystLogger::log(LogLevel::Error, &full_msg);
    }));
}

#[macro_export]
macro_rules! cata_log {
    ($level:ident, $msg:expr) => {{
        let src_loc = if module_path!().len() > 25 {
            format!("{}:{}::{}", file!().split('/').last().unwrap_or(file!()), line!(), module_path!().split("::").last().unwrap_or(""))
        } else {
            format!("{}:{}", file!(), line!())
        };

        let full_msg = format!("[{}] {}", src_loc, $msg);
        $crate::services::logger::CatalystLogger::log($crate::services::logger::LogLevel::$level, &full_msg);
    }};
}
