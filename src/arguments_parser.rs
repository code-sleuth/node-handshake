//! Command-line argument parsing and configuration management
//!
//! This module provides CLI argument parsing using clap for configuring
//! the Solana handshake client and gossip node behavior.

use clap::Parser;
use std::str::FromStr;
use std::time::Duration;

/// Command-line arguments for the Solana P2P handshake application
#[derive(Parser, Debug, Clone)]
#[command(
    name = "p2p_solana_handshake",
    version = "0.1.0",
    about = "Solana P2P handshake client and gossip node",
    long_about = "A tool for performing handshake protocols with Solana gossip nodes and participating in the Solana P2P network."
)]
pub struct Args {
    /// Local address to bind the gossip node (e.g., "0.0.0.0:8000")
    #[arg(
        short = 'b',
        long = "bind",
        default_value = "0.0.0.0:0",
        help = "Local address to bind for UDP communication"
    )]
    pub bind_address: String,

    /// Solana network to connect to
    #[arg(
        short = 'n',
        long = "network",
        default_value = "localnet",
        help = "Solana network ID (localnet, testnet, devnet, mainnet-beta)"
    )]
    pub network_id: NetworkId,

    /// Remote peer addresses to connect to (can be specified multiple times)
    #[arg(
        short = 'p',
        long = "peers",
        help = "Remote peer addresses to perform handshakes with (e.g., 127.0.0.1:8001 or gossip-server:8000)"
    )]
    pub peer_addresses: Vec<String>,

    /// Operation timeout in seconds
    #[arg(
        short = 't',
        long = "timeout",
        default_value = "30",
        help = "Timeout for network operations in seconds"
    )]
    pub timeout_secs: u16,

    /// Maximum retry attempts for failed handshakes
    #[arg(
        short = 'r',
        long = "max-retries",
        default_value = "3",
        help = "Maximum number of retry attempts for failed handshakes"
    )]
    pub max_retries: u8,

    /// Application mode
    #[arg(
        short = 'm',
        long = "mode",
        default_value = "client",
        help = "Application mode: client (perform handshakes) or server (listen for handshakes)"
    )]
    pub mode: ApplicationMode,

    /// Log level for application output
    #[arg(
        short = 'l',
        long = "log-level",
        default_value = "info",
        help = "Log level (trace, debug, info, warn, error)"
    )]
    pub log_level: LogLevel,

    /// Output log format
    #[arg(
        long = "log-format",
        default_value = "pretty",
        help = "Log output format (pretty, json)"
    )]
    pub log_format: LogFormat,

    /// Run in continuous mode (keep the application running)
    #[arg(
        short = 'c',
        long = "continuous",
        help = "Run in continuous mode, keeping the application running"
    )]
    pub continuous: bool,

    /// Interval between handshake attempts in continuous mode (seconds)
    #[arg(
        short = 'i',
        long = "interval",
        default_value = "60",
        help = "Interval between handshake attempts in continuous mode (seconds)"
    )]
    pub interval_secs: u16,
}

impl Args {
    /// Get the network timeout as Duration
    pub fn network_timeout(&self) -> Duration {
        Duration::from_secs(self.timeout_secs.into())
    }

    /// Get the continuous mode interval as a Duration  
    pub fn continuous_interval(&self) -> Duration {
        Duration::from_secs(self.interval_secs.into())
    }

    /// Validate the provided arguments
    pub fn validate(&self) -> Result<(), String> {
        // Validate network timeout
        if self.timeout_secs == 0 {
            return Err("Timeout must be greater than 0 seconds".to_string());
        }

        if self.timeout_secs > 300 {
            return Err("Timeout cannot exceed 300 seconds (5 minutes)".to_string());
        }

        // Validate max retries
        if self.max_retries > 10 {
            return Err("Maximum retries cannot exceed 10".to_string());
        }

        // Validate peer addresses for client mode
        if matches!(self.mode, ApplicationMode::Client) && self.peer_addresses.is_empty() {
            return Err("Client mode requires at least one peer address (--peers)".to_string());
        }

        // Validate peer address formats (can be IP:port or hostname:port)
        for addr in &self.peer_addresses {
            if !addr.contains(':') {
                return Err(format!(
                    "Invalid peer address format '{}'. Must include port (e.g., host:8000)",
                    addr
                ));
            }
        }

        // Validate continuous mode interval
        if self.continuous && self.interval_secs < 10 {
            return Err("Continuous mode interval must be at least 10 seconds".to_string());
        }

        Ok(())
    }
}

/// Supported Solana network identifiers
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetworkId {
    /// Local development network
    Localnet,
    /// Solana testnet
    Testnet,
    /// Solana devnet  
    Devnet,
    /// Solana mainnet-beta
    MainnetBeta,
}

impl FromStr for NetworkId {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use crate::solana::network;

        match s.to_lowercase().as_str() {
            s if s == network::LOCALNET || s == "local" => Ok(NetworkId::Localnet),
            s if s == network::TESTNET || s == "test" => Ok(NetworkId::Testnet),
            s if s == network::DEVNET || s == "dev" => Ok(NetworkId::Devnet),
            s if s == network::MAINNET_BETA || s == "mainnet" => Ok(NetworkId::MainnetBeta),
            _ => Err(format!(
                "Invalid network ID: '{}'. Valid options: {}, {}, {}, {}",
                s,
                network::LOCALNET,
                network::TESTNET,
                network::DEVNET,
                network::MAINNET_BETA
            )),
        }
    }
}

impl std::fmt::Display for NetworkId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use crate::solana::network;

        let network_str = match self {
            NetworkId::Localnet => network::LOCALNET,
            NetworkId::Testnet => network::TESTNET,
            NetworkId::Devnet => network::DEVNET,
            NetworkId::MainnetBeta => network::MAINNET_BETA,
        };
        write!(f, "{}", network_str)
    }
}

/// Application operating mode
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApplicationMode {
    /// Client mode - initiate handshakes with peers
    Client,
    /// Server mode - listen and respond to handshakes
    Server,
}

impl FromStr for ApplicationMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "client" | "c" => Ok(ApplicationMode::Client),
            "server" | "s" | "listen" => Ok(ApplicationMode::Server),
            _ => Err(format!(
                "Invalid application mode: '{}'. Valid options: client, server",
                s
            )),
        }
    }
}

/// Log level configuration
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl FromStr for LogLevel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "trace" => Ok(LogLevel::Trace),
            "debug" => Ok(LogLevel::Debug),
            "info" => Ok(LogLevel::Info),
            "warn" | "warning" => Ok(LogLevel::Warn),
            "error" => Ok(LogLevel::Error),
            _ => Err(format!(
                "Invalid log level: '{}'. Valid options: trace, debug, info, warn, error",
                s
            )),
        }
    }
}

impl From<LogLevel> for tracing::Level {
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::Trace => tracing::Level::TRACE,
            LogLevel::Debug => tracing::Level::DEBUG,
            LogLevel::Info => tracing::Level::INFO,
            LogLevel::Warn => tracing::Level::WARN,
            LogLevel::Error => tracing::Level::ERROR,
        }
    }
}

/// Log output format
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogFormat {
    /// Pretty formatted logs for development
    Pretty,
    /// JSON formatted logs for production
    Json,
}

impl FromStr for LogFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pretty" | "text" | "human" => Ok(LogFormat::Pretty),
            "json" => Ok(LogFormat::Json),
            _ => Err(format!(
                "Invalid log format: '{}'. Valid options: pretty, json",
                s
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_id_parsing() {
        assert_eq!(
            "localnet".parse::<NetworkId>().unwrap(),
            NetworkId::Localnet
        );
        assert_eq!("testnet".parse::<NetworkId>().unwrap(), NetworkId::Testnet);
        assert_eq!(
            "mainnet-beta".parse::<NetworkId>().unwrap(),
            NetworkId::MainnetBeta
        );
        assert!("invalid".parse::<NetworkId>().is_err());
    }

    #[test]
    fn test_application_mode_parsing() {
        assert_eq!(
            "client".parse::<ApplicationMode>().unwrap(),
            ApplicationMode::Client
        );
        assert_eq!(
            "server".parse::<ApplicationMode>().unwrap(),
            ApplicationMode::Server
        );
        assert!("invalid".parse::<ApplicationMode>().is_err());
    }

    #[test]
    fn test_args_validation() {
        let mut args = Args {
            bind_address: "0.0.0.0:8000".to_string(),
            network_id: NetworkId::Localnet,
            peer_addresses: vec!["127.0.0.1:8001".to_string()],
            timeout_secs: 30,
            max_retries: 3,
            mode: ApplicationMode::Client,
            log_level: LogLevel::Info,
            log_format: LogFormat::Pretty,
            continuous: false,
            interval_secs: 60,
        };

        assert!(args.validate().is_ok());

        // Test timeout validation
        args.timeout_secs = 0;
        assert!(args.validate().is_err());

        args.timeout_secs = 400;
        assert!(args.validate().is_err());

        // Test client mode peer validation
        args.timeout_secs = 30;
        args.peer_addresses.clear();
        assert!(args.validate().is_err());
    }

    #[test]
    fn test_args_validation_max_retries() {
        let mut args = create_valid_args();

        // Test max retries validation
        args.max_retries = 11; // Over the limit
        let result = args.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("Maximum retries cannot exceed 10"));

        // Valid retry count
        args.max_retries = 5;
        assert!(args.validate().is_ok());
    }

    #[test]
    fn test_args_validation_server_mode() {
        let mut args = create_valid_args();
        args.mode = ApplicationMode::Server;
        args.peer_addresses.clear(); // Server doesn't need peer addresses

        assert!(args.validate().is_ok());
    }

    #[test]
    fn test_args_validation_continuous_mode() {
        let mut args = create_valid_args();
        args.continuous = true;
        args.interval_secs = 5; // Too short

        let result = args.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("Continuous mode interval must be at least 10 seconds"));

        // Valid interval
        args.interval_secs = 30;
        assert!(args.validate().is_ok());
    }

    #[test]
    fn test_network_timeout_conversion() {
        let args = create_valid_args();
        let timeout = args.network_timeout();
        assert_eq!(timeout, Duration::from_secs(30));
    }

    #[test]
    fn test_continuous_interval_conversion() {
        let args = create_valid_args();
        let interval = args.continuous_interval();
        assert_eq!(interval, Duration::from_secs(60));
    }

    #[test]
    fn test_log_level_from_str() {
        assert_eq!("trace".parse::<LogLevel>().unwrap(), LogLevel::Trace);
        assert_eq!("debug".parse::<LogLevel>().unwrap(), LogLevel::Debug);
        assert_eq!("info".parse::<LogLevel>().unwrap(), LogLevel::Info);
        assert_eq!("warn".parse::<LogLevel>().unwrap(), LogLevel::Warn);
        assert_eq!("warning".parse::<LogLevel>().unwrap(), LogLevel::Warn);
        assert_eq!("error".parse::<LogLevel>().unwrap(), LogLevel::Error);

        assert!("invalid".parse::<LogLevel>().is_err());
    }

    #[test]
    fn test_log_format_from_str() {
        assert_eq!("pretty".parse::<LogFormat>().unwrap(), LogFormat::Pretty);
        assert_eq!("text".parse::<LogFormat>().unwrap(), LogFormat::Pretty);
        assert_eq!("human".parse::<LogFormat>().unwrap(), LogFormat::Pretty);
        assert_eq!("json".parse::<LogFormat>().unwrap(), LogFormat::Json);

        assert!("invalid".parse::<LogFormat>().is_err());
    }

    #[test]
    fn test_log_level_to_tracing_level() {
        use tracing::Level;

        assert_eq!(Level::from(LogLevel::Trace), Level::TRACE);
        assert_eq!(Level::from(LogLevel::Debug), Level::DEBUG);
        assert_eq!(Level::from(LogLevel::Info), Level::INFO);
        assert_eq!(Level::from(LogLevel::Warn), Level::WARN);
        assert_eq!(Level::from(LogLevel::Error), Level::ERROR);
    }

    #[test]
    fn test_network_id_display() {
        assert_eq!(format!("{}", NetworkId::Localnet), "localnet");
        assert_eq!(format!("{}", NetworkId::Testnet), "testnet");
        assert_eq!(format!("{}", NetworkId::Devnet), "devnet");
        assert_eq!(format!("{}", NetworkId::MainnetBeta), "mainnet-beta");
    }

    #[test]
    fn test_all_enum_variants_covered() {
        // Ensure we handle all NetworkId variants
        for network in [
            NetworkId::Localnet,
            NetworkId::Testnet,
            NetworkId::Devnet,
            NetworkId::MainnetBeta,
        ] {
            let network_str = format!("{}", network);
            assert!(!network_str.is_empty());
            assert_eq!(network_str.parse::<NetworkId>().unwrap(), network);
        }

        // Ensure we handle all ApplicationMode variants
        for mode in [ApplicationMode::Client, ApplicationMode::Server] {
            match mode {
                ApplicationMode::Client => {
                    assert_eq!("client".parse::<ApplicationMode>().unwrap(), mode);
                }
                ApplicationMode::Server => {
                    assert_eq!("server".parse::<ApplicationMode>().unwrap(), mode);
                }
            }
        }

        // Ensure we handle all LogLevel variants
        for level in [
            LogLevel::Trace,
            LogLevel::Debug,
            LogLevel::Info,
            LogLevel::Warn,
            LogLevel::Error,
        ] {
            let level_str = match level {
                LogLevel::Trace => "trace",
                LogLevel::Debug => "debug",
                LogLevel::Info => "info",
                LogLevel::Warn => "warn",
                LogLevel::Error => "error",
            };
            assert_eq!(level_str.parse::<LogLevel>().unwrap(), level);
        }
    }

    fn create_valid_args() -> Args {
        Args {
            bind_address: "0.0.0.0:8000".to_string(),
            network_id: NetworkId::Localnet,
            peer_addresses: vec!["127.0.0.1:8001".to_string()],
            timeout_secs: 30,
            max_retries: 3,
            mode: ApplicationMode::Client,
            log_level: LogLevel::Info,
            log_format: LogFormat::Pretty,
            continuous: false,
            interval_secs: 60,
        }
    }
}
