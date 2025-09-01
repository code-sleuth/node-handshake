//! Telemetry and logging infrastructure
//!
//! This module provides a structured logging setup using tracing with support
//! for both development-friendly and production-ready output formats.

use crate::arguments_parser::{LogFormat, LogLevel};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Initialize the telemetry and logging infrastructure
///
/// This function sets up structured logging using the tracing ecosystem with
/// configurable output formats and log levels.
pub fn setup_telemetry(log_level: LogLevel, log_format: LogFormat) -> Result<(), String> {
    // Create base filter from log level
    let base_filter = EnvFilter::from_default_env().add_directive(
        format!("p2p_solana_handshake={}", level_to_str(&log_level))
            .parse()
            .map_err(|e| format!("Invalid log level directive: {}", e))?,
    );

    match log_format {
        LogFormat::Pretty => setup_pretty_logging(base_filter),
        LogFormat::Json => setup_json_logging(base_filter),
    }
}

/// Setup pretty-formatted logging for development
fn setup_pretty_logging(filter: EnvFilter) -> Result<(), String> {
    let formatting_layer = fmt::layer()
        .with_target(true)
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_file(true)
        .with_line_number(true)
        .with_level(true)
        .with_ansi(true)
        .compact();

    tracing_subscriber::registry()
        .with(filter)
        .with(formatting_layer)
        .try_init()
        .map_err(|e| format!("Failed to initialize pretty logging: {}", e))?;

    tracing::info!("Pretty logging initialized");
    Ok(())
}

/// Setup JSON-formatted logging for production
fn setup_json_logging(filter: EnvFilter) -> Result<(), String> {
    // Create a Bunyan-style JSON formatter for structured logs
    let bunyan_formatting_layer = tracing_bunyan_formatter::BunyanFormattingLayer::new(
        "p2p_solana_handshake".to_string(),
        std::io::stdout,
    );

    let json_layer = tracing_bunyan_formatter::JsonStorageLayer;

    tracing_subscriber::registry()
        .with(filter)
        .with(json_layer)
        .with(bunyan_formatting_layer)
        .try_init()
        .map_err(|e| format!("Failed to initialize JSON logging: {}", e))?;

    tracing::info!("JSON logging initialized");
    Ok(())
}

/// Convert LogLevel to string representation for EnvFilter
fn level_to_str(level: &LogLevel) -> &'static str {
    match level {
        LogLevel::Trace => "trace",
        LogLevel::Debug => "debug",
        LogLevel::Info => "info",
        LogLevel::Warn => "warn",
        LogLevel::Error => "error",
    }
}

/// Create a correlation ID for tracing requests across services
pub fn generate_correlation_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();

    // Simple correlation ID based on timestamp and random component
    let random_component: u32 = rand::random();
    format!("{:x}{:08x}", timestamp, random_component)
}

/// Structured logging macros with correlation IDs
#[macro_export]
macro_rules! log_with_context {
    ($level:ident, correlation_id = $correlation_id:expr, $($field:tt)*) => {
        tracing::$level!(correlation_id = $correlation_id, $($field)*);
    };
}

/// Helper functions to log errors with their complete error chain
pub fn log_error_chain(error: &dyn std::error::Error) {
    let mut chain = Vec::new();
    let mut source = Some(error);

    while let Some(err) = source {
        chain.push(err.to_string());
        source = err.source();
    }

    tracing::error!(
        error.message = %error,
        error.chain = ?chain,
        "Error occurred with full chain"
    );
}

/// Log successful operations with timing information
pub fn log_operation_success(operation: &str, duration: std::time::Duration) {
    tracing::info!(
        operation = operation,
        duration_ms = duration.as_millis(),
        "Operation completed successfully"
    );
}

/// Log failed operations with error details
pub fn log_operation_failure(
    operation: &str,
    error: &dyn std::error::Error,
    duration: std::time::Duration,
) {
    tracing::error!(
        operation = operation,
        duration_ms = duration.as_millis(),
        error.message = %error,
        "Operation failed"
    );
    log_error_chain(error);
}

/// Configuration for telemetry in different environments
pub struct TelemetryConfig {
    pub service_name: String,
    pub service_version: String,
    pub environment: String,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            service_name: "p2p_solana_handshake".to_string(),
            service_version: env!("CARGO_PKG_VERSION").to_string(),
            environment: std::env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string()),
        }
    }
}

impl TelemetryConfig {
    /// Create telemetry config from environment variables
    pub fn from_env() -> Self {
        Self {
            service_name: std::env::var("SERVICE_NAME")
                .unwrap_or_else(|_| "p2p_solana_handshake".to_string()),
            service_version: std::env::var("SERVICE_VERSION")
                .unwrap_or_else(|_| env!("CARGO_PKG_VERSION").to_string()),
            environment: std::env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string()),
        }
    }

    /// Check if running in a production environment
    pub fn is_production(&self) -> bool {
        matches!(self.environment.as_str(), "production" | "prod")
    }

    /// Get recommended log format for this environment
    pub fn recommended_log_format(&self) -> LogFormat {
        if self.is_production() {
            LogFormat::Json
        } else {
            LogFormat::Pretty
        }
    }

    /// Get a recommended log level for this environment
    pub fn recommended_log_level(&self) -> LogLevel {
        match self.environment.as_str() {
            "production" | "prod" => LogLevel::Info,
            "staging" | "stage" => LogLevel::Debug,
            _ => LogLevel::Debug,
        }
    }
}
