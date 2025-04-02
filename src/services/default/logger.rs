use chrono::{Local, Utc};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::panic::{self, PanicHookInfo};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warning,
    Error,
}

impl LogLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Trace => "TRACE",
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warning => "WARNING",
            LogLevel::Error => "ERROR",
        }
    }

    pub fn color_code(&self) -> &'static str {
        match self {
            LogLevel::Trace => "\x1b[90m",   // Gray
            LogLevel::Debug => "\x1b[36m",   // Cyan
            LogLevel::Info => "\x1b[32m",    // Green
            LogLevel::Warning => "\x1b[33m", // Yellow
            LogLevel::Error => "\x1b[31m",   // Red
        }
    }
}

pub struct CatalystLogger;

impl CatalystLogger {
    pub fn log(level: LogLevel, message: &str) {
        let log_dir = "storage/logs";
        if let Err(e) = fs::create_dir_all(log_dir) {
            eprintln!("Failed to create log directory: {}", e);
            return;
        }
        let file_path = Path::new(log_dir).join(format!("{}.log", level.as_str().to_lowercase()));

        // Get current time as Unix timestamp
        let unix_ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();

        // Get local time with timezone
        let local_time = Local::now().format("%Y-%m-%d %H:%M:%S %Z").to_string();

        // Create timestamp string with both Unix timestamp and human-readable time
        let timestamp = format!("{}-{}", unix_ts, local_time);

        // Create colorized log entry with only the level colored (like tracing crate)
        let color_code = level.color_code();
        let reset_code = "\x1b[0m";
        let colored_log_entry = format!("{} [{}{}{}] {}\n", 
            timestamp, color_code, level.as_str(), reset_code, message);

        // Create plain log entry for file (no color codes)
        let file_log_entry = format!("{} [{}] {}\n", timestamp, level.as_str(), message);

        // Write to file
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

        // Get current time as Unix timestamp
        let unix_ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();

        // Get local time with timezone
        let local_time = Local::now().format("%Y-%m-%d %H:%M:%S %Z").to_string();

        // Create timestamp string with both Unix timestamp and human-readable time
        let timestamp = format!("{}-{}", unix_ts, local_time);

        // Create colorized log entry with only the level colored (like tracing crate)
        let color_code = level.color_code();
        let reset_code = "\x1b[0m";
        let colored_log_entry = format!("{} [{}{}{}] {}\n", 
            timestamp, color_code, level.as_str(), reset_code, message);

        // Create plain log entry for file (no color codes)
        let file_log_entry = format!("{} [{}] {}\n", timestamp, level.as_str(), message);

        // Write to file
        if let Err(e) = OpenOptions::new().append(true).create(true).open(path).and_then(|mut file| file.write_all(file_log_entry.as_bytes())) {
            eprintln!("Failed to write log to {}: {}", path.display(), e);
        }

        // Also print to console for immediate feedback
        print!("{}", colored_log_entry);
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
        let full_msg = format!("🚨 Panic at {}: {}", location, payload);
        CatalystLogger::log(LogLevel::Error, &full_msg);
    }));
}

#[macro_export]
macro_rules! cata_log {
    ($level:ident, $msg:expr) => {{
        let level_icon = match stringify!($level) {
            "Trace" => "🔍",  // Magnifying glass
            "Debug" => "🐞",  // Bug
            "Info" => "ℹ️",   // Info
            "Warning" => "⚠️", // Warning
            "Error" => "❌",   // Error
            _ => "•",
        };
        
        // Formatted source location for easier debugging
        let src_loc = if module_path!().len() > 25 {
            format!("{}:{}::{}", 
                file!().split('/').last().unwrap_or(file!()), 
                line!(),
                module_path!().split("::").last().unwrap_or(""))
        } else {
            format!("{}:{}", file!(), line!())
        };
        
        let full_msg = format!("{} [{}] {}", level_icon, src_loc, $msg);
        $crate::services::logger::CatalystLogger::log($crate::services::logger::LogLevel::$level, &full_msg);
    }};
}
