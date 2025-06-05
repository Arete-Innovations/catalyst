use std::{
    fs::{self, OpenOptions},
    io::Write,
    panic::{self, PanicHookInfo, Location},
    path::Path,
    process, thread,
    time::{SystemTime, UNIX_EPOCH},
};

use backtrace::Backtrace;
use chrono::Local;

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

    #[track_caller]
    pub fn log_with_caller(level: LogLevel, message: &str) {
        let location = Location::caller();
        let src_loc = format!("{}:{}::{}", 
            location.file().split('/').last().unwrap_or(location.file()), 
            location.line(),
            location.file().split('/').nth_back(1).unwrap_or("unknown")
        );
        
        let full_msg = format!("[{}] {}", src_loc, message);
        Self::log(level, &full_msg);
    }

    #[track_caller]
    pub fn log_with_caller_and_context(level: LogLevel, message: &str, context: &str) {
        let location = Location::caller();
        let src_loc = format!("{}:{}::{}", 
            location.file().split('/').last().unwrap_or(location.file()), 
            location.line(),
            location.file().split('/').nth_back(1).unwrap_or("unknown")
        );
        
        let enhanced_msg = format!("{} → {}", message, context);
        let full_msg = format!("[{}] {}", src_loc, enhanced_msg);
        Self::log(level, &full_msg);
    }

    #[track_caller]
    pub fn log_with_caller_and_data(level: LogLevel, message: &str, data: &str) {
        let location = Location::caller();
        let src_loc = format!("{}:{}::{}", 
            location.file().split('/').last().unwrap_or(location.file()), 
            location.line(),
            location.file().split('/').nth_back(1).unwrap_or("unknown")
        );
        
        let enhanced_msg = format!("{} | data: {}", message, data);
        let full_msg = format!("[{}] {}", src_loc, enhanced_msg);
        Self::log(level, &full_msg);
    }

    #[track_caller]
    pub fn log_with_caller_context_and_data(level: LogLevel, message: &str, context: &str, data: &str) {
        let location = Location::caller();
        let src_loc = format!("{}:{}::{}", 
            location.file().split('/').last().unwrap_or(location.file()), 
            location.line(),
            location.file().split('/').nth_back(1).unwrap_or("unknown")
        );
        
        let enhanced_msg = format!("{} → {} | data: {}", message, context, data);
        let full_msg = format!("[{}] {}", src_loc, enhanced_msg);
        Self::log(level, &full_msg);
    }
}

pub fn capture_call_stack() -> (Vec<String>, u128) {
    let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();

    // Force backtrace capture for normal operation

    let backtrace = Backtrace::new();
    let backtrace_str = format!("{:?}", backtrace);

    let call_stack = parse_backtrace_simple(&backtrace_str);
    let end_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();

    let trace_time_ns = end_time - start_time;
    (call_stack, trace_time_ns)
}

fn parse_backtrace_simple(backtrace_str: &str) -> Vec<String> {
    let mut call_stack = Vec::new();

    // Extract all functions, then filter out noise
    for line in backtrace_str.lines() {
        // Try all extraction methods for each line
        if let Some(function_info) = extract_simple_function_info(line) {
            if !should_skip_function(&function_info) {
                call_stack.push(function_info);
            }
        } else if line.contains("src/") && line.contains(".rs") {
            if let Some(file_info) = extract_from_src_path(line) {
                if !should_skip_function(&file_info) {
                    call_stack.push(file_info);
                }
            }
        } else if let Some(any_function) = extract_any_useful_function(line) {
            if !should_skip_function(&any_function) {
                call_stack.push(any_function);
            }
        }
    }

    // Remove duplicates while preserving order
    call_stack.dedup();

    // Reverse to show call order (caller -> callee)
    call_stack.reverse();

    call_stack
}

fn extract_any_useful_function(line: &str) -> Option<String> {
    // Look for any function that might be from your application
    // Pattern: "some_module::some_function"
    if line.contains("::") {
        let trimmed = line.trim();

        // Look for patterns like "bootstrap::initialize" or "auth::login_handler"
        if let Some(double_colon) = trimmed.find("::") {
            let before = &trimmed[..double_colon];
            let after = &trimmed[double_colon + 2..];

            // Extract the last module name
            let module = if let Some(last_colon) = before.rfind("::") {
                &before[last_colon + 2..]
            } else if let Some(space) = before.rfind(' ') {
                &before[space + 1..]
            } else {
                before
            };

            // Extract function name (stop at space or special chars)
            let function = after.split_whitespace().next().unwrap_or(after).split("::").next().unwrap_or(after);

            // Only return if it looks like application code
            if !module.is_empty() && !function.is_empty() && module.len() > 1 && function.len() > 1 && !module.contains('<') && !function.contains('<') {
                return Some(format!("{}::{}", module, function));
            }
        }
    }

    None
}

fn extract_from_src_path(line: &str) -> Option<String> {
    // Extract from lines like "   at src/services/auth.rs:123:45"
    if let Some(src_pos) = line.find("src/") {
        let after_src = &line[src_pos + 4..];
        if let Some(rs_pos) = after_src.find(".rs") {
            let before_rs = &after_src[..rs_pos];
            let after_rs = &after_src[rs_pos + 3..]; // Skip ".rs"

            // Extract line number - fix bounds checking
            let line_number = if let Some(colon_pos) = after_rs.find(':') {
                if colon_pos > 0 && after_rs.len() > 1 {
                    let line_part = &after_rs[1..colon_pos]; // Skip first ':'
                    line_part.parse::<u32>().ok()
                } else {
                    None
                }
            } else {
                None
            };

            let parts: Vec<&str> = before_rs.split('/').collect();
            if let Some(filename) = parts.last() {
                let module = if parts.len() > 1 { parts[parts.len() - 2] } else { "main" };

                if let Some(line_num) = line_number {
                    return Some(format!("{}::{}:{}", module, filename, line_num));
                } else {
                    return Some(format!("{}::{}", module, filename));
                }
            }
        }
    }
    None
}

fn extract_line_number_from_line(line: &str) -> Option<u32> {
    // Look for patterns like ":123:" or ":123" at the end
    if let Some(src_pos) = line.find("src/") {
        let after_src = &line[src_pos..];
        if let Some(rs_pos) = after_src.find(".rs:") {
            let after_rs = &after_src[rs_pos + 4..]; // Skip ".rs:"
            if let Some(colon_pos) = after_rs.find(':') {
                let line_part = &after_rs[..colon_pos];
                return line_part.parse::<u32>().ok();
            } else {
                // No second colon, take everything after .rs:
                return after_rs.trim().parse::<u32>().ok();
            }
        }
    }
    None
}

fn extract_line_number_from_path(path: &str) -> Option<u32> {
    // Extract line number from paths like "src/services/auth.rs:123:45"
    if let Some(colon_pos) = path.rfind(':') {
        let before_colon = &path[..colon_pos];
        if let Some(prev_colon) = before_colon.rfind(':') {
            let line_part = &before_colon[prev_colon + 1..];
            return line_part.parse::<u32>().ok();
        }
    }
    None
}

fn extract_simple_function_info(line: &str) -> Option<String> {
    // Look for any catalyst:: references first - enhanced with line number extraction
    if line.contains("catalyst::") {
        if let Some(catalyst_start) = line.find("catalyst::") {
            let after_catalyst = &line[catalyst_start + 10..]; // Skip "catalyst::"

            // Look for line number info in the entire line
            let line_number = extract_line_number_from_line(line);

            // Remove hash suffix and clean up
            let clean_path = if let Some(hash_pos) = after_catalyst.find("::h") {
                &after_catalyst[..hash_pos]
            } else if let Some(space_pos) = after_catalyst.find(' ') {
                &after_catalyst[..space_pos]
            } else {
                after_catalyst
            };

            let parts: Vec<&str> = clean_path.split("::").collect();

            // Try to get meaningful module::function
            if parts.len() >= 2 {
                let module = parts[parts.len() - 2];
                let function = parts[parts.len() - 1];

                // Skip if it's just implementation details
                if function.starts_with('<') || function.contains("{{") || function.is_empty() {
                    return None;
                }

                if let Some(line_num) = line_number {
                    return Some(format!("{}::{}:{}", module, function, line_num));
                } else {
                    return Some(format!("{}::{}", module, function));
                }
            } else if parts.len() == 1 && !parts[0].is_empty() && !parts[0].starts_with('<') {
                if let Some(line_num) = line_number {
                    return Some(format!("catalyst::{}:{}", parts[0], line_num));
                } else {
                    return Some(format!("catalyst::{}", parts[0]));
                }
            }
        }
    }

    // Look for function patterns with line info - enhanced extraction
    if line.contains("::") && !line.contains("std::") && !line.contains("core::") {
        let trimmed = line.trim();

        // Enhanced pattern matching for "module::function at src/file.rs:123:45"
        if let Some(at_pos) = trimmed.find(" at ") {
            let before_at = &trimmed[..at_pos];
            let after_at = &trimmed[at_pos + 4..];

            // Extract line number from the "at" part
            let line_number = if after_at.contains("src/") { extract_line_number_from_path(after_at) } else { None };

            // Parse the function part
            if let Some(colon_pos) = before_at.rfind("::") {
                let before_colon = &before_at[..colon_pos];
                let after_colon = &before_at[colon_pos + 2..];

                if let Some(last_module_pos) = before_colon.rfind("::") {
                    let module = &before_colon[last_module_pos + 2..];
                    let function = after_colon.split_whitespace().next().unwrap_or(after_colon);

                    if !module.is_empty() && !function.is_empty() && !function.starts_with('<') {
                        if let Some(line_num) = line_number {
                            return Some(format!("{}::{}:{}", module, function, line_num));
                        } else {
                            return Some(format!("{}::{}", module, function));
                        }
                    }
                }
            }
        }

        // Fallback: simple module::function extraction with line number search
        if let Some(colon_pos) = trimmed.rfind("::") {
            let before_colon = &trimmed[..colon_pos];
            let after_colon = &trimmed[colon_pos + 2..];

            if let Some(last_module_pos) = before_colon.rfind("::") {
                let module = &before_colon[last_module_pos + 2..];
                let function = after_colon.split_whitespace().next().unwrap_or(after_colon);

                if !module.is_empty() && !function.is_empty() && !function.starts_with('<') {
                    // Look for line numbers anywhere in the line
                    let line_number = extract_line_number_from_line(line);

                    if let Some(line_num) = line_number {
                        return Some(format!("{}::{}:{}", module, function, line_num));
                    } else {
                        return Some(format!("{}::{}", module, function));
                    }
                }
            }
        }
    }

    None
}

fn should_skip_function(function_info: &str) -> bool {
    let skip_patterns = [
        "logger::",
        "cata_log",
        "backtrace::",
        "std::",
        "core::",
        "alloc::",
        "rust_begin_unwind",
        "rust_panic",
        "{{closure}}",
        "<unknown>",
        // Skip async runtime noise
        "Runtime::",
        "Inner::",
        "Task::",
        "UnownedTask",
        "RawTask::",
        "raw::",
        "runtime::",
        "MultiThread::",
        "block_on",
        "enter_runtime",
        "poll",
        "worker::",
        "context::",
        "Scoped",
        "Context::",
        "coop::",
        "budget",
        "with_budget",
        // Skip massive amounts of Rust runtime noise
        "src::rt",
        "main::rt",
        "main::panic",
        "main::panicking",
        "ops::function",
        "sys::backtrace",
        "main::main",
        "src::lib",
        "multi_thread::",
        "future::future",
        "panic::",
        "panicking",
        "unwind_safe",
        "src::panic",
        "src::panicking",
        "thread::",
        "unix::thread",
        "main::boxed",
        "task::",
        "blocking::",
        "harness",
        "LocalNotified",
        "catch_unwind",
        "OnceCell",
        "Lazy<T,F>",
        "src::imp_std",
        "imp::",
        "src::server",
        "src::rkt",
        "fairing::",
        "join_all",
        "maybe_done",
        // Skip low-level Rocket internals
        "rocket::",
        "_Handler::",
        "Outcome::",
        // Skip tokio internals
        "tokio::",
        "futures::",
        // Skip if it's just the main function (not useful for tracing)
        "catalyst::main",
        // Skip generic type noise
        "<impl",
        "{{",
        // Skip logger recursion
        "default::logger",
    ];

    for pattern in &skip_patterns {
        if function_info.contains(pattern) {
            return true;
        }
    }

    false
}

pub fn format_with_trace(message: &str, call_stack: &[String]) -> String {
    if call_stack.is_empty() || call_stack.iter().all(|s| s.trim().is_empty()) {
        return message.to_string();
    }

    // Filter out empty or invalid entries
    let valid_calls: Vec<&String> = call_stack.iter().filter(|s| !s.trim().is_empty() && !s.contains("<impl")).collect();

    if valid_calls.is_empty() {
        return message.to_string();
    }

    let context_trace = valid_calls.iter().enumerate().map(|(i, func)| format!("    {}. {}", i + 1, func)).collect::<Vec<_>>().join("\n");

    format!("{}\n  Call trace:\n{}", message, context_trace)
}

pub fn format_with_context(message: &str, context: &str, call_stack: &[String]) -> String {
    let base_msg = format!("{} [Context: {}]", message, context);
    format_with_trace(&base_msg, call_stack)
}

pub fn get_execution_context() -> String {
    let thread_name = thread::current().name().map(|n| n.to_string()).unwrap_or_else(|| "unnamed".to_string());

    format!("thread:{}", thread_name)
}

fn format_time(nanoseconds: u128) -> String {
    if nanoseconds < 1_000 {
        format!("{}ns", nanoseconds)
    } else if nanoseconds < 1_000_000 {
        format!("{:.1}μs", nanoseconds as f64 / 1_000.0)
    } else if nanoseconds < 1_000_000_000 {
        format!("{:.1}ms", nanoseconds as f64 / 1_000_000.0)
    } else {
        format!("{:.2}s", nanoseconds as f64 / 1_000_000_000.0)
    }
}

pub fn format_with_context_and_trace_timed(message: &str, context: &str, call_stack: &[String], trace_time_ns: u128) -> String {
    // Filter out meaningless call stack entries
    let meaningful_stack: Vec<&String> = call_stack
        .iter()
        .filter(|s| !s.trim().is_empty() && !s.contains("no_application_functions_found") && !s.contains("trace_parsing_failed"))
        .collect();

    let formatted_time = format_time(trace_time_ns);

    if meaningful_stack.is_empty() {
        // Clean single-line format for simple logs
        format!("{} → {} • {}", message, context, formatted_time)
    } else {
        // Single-line format with arrow to trace
        let trace_chain = meaningful_stack.iter().map(|s| s.as_str()).collect::<Vec<&str>>().join(" → ");
        format!("{} → {} • {} → {}", message, context, formatted_time, trace_chain)
    }
}

pub fn format_with_context_and_trace(message: &str, context: &str, call_stack: &[String]) -> String {
    format_with_context_and_trace_timed(message, context, call_stack, 0)
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
    ($level:ident, $msg:expr) => {
        $crate::services::logger::CatalystLogger::log_with_caller(
            $crate::services::logger::LogLevel::$level, 
            &$msg.to_string()
        )
    };

    ($level:ident, $msg:expr, context: $context:expr) => {
        $crate::services::logger::CatalystLogger::log_with_caller_and_context(
            $crate::services::logger::LogLevel::$level, 
            &$msg.to_string(),
            &$context.to_string()
        )
    };

    ($level:ident, $msg:expr, data: $data:expr) => {
        $crate::services::logger::CatalystLogger::log_with_caller_and_data(
            $crate::services::logger::LogLevel::$level, 
            &$msg.to_string(),
            &$data.to_string()
        )
    };

    ($level:ident, $msg:expr, context: $context:expr, data: $data:expr) => {
        $crate::services::logger::CatalystLogger::log_with_caller_context_and_data(
            $crate::services::logger::LogLevel::$level, 
            &$msg.to_string(),
            &$context.to_string(),
            &$data.to_string()
        )
    };
}
