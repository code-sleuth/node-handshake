//! Domain models and data structures for Solana network communication
//!
//! This module defines the core data types, error handling, and serialization
//! formats used throughout the Solana P2P networking implementation.

use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use std::net::SocketAddr;
use thiserror::Error;

/// Solana network identifiers as constants
pub mod network {
    pub const LOCALNET: &str = "localnet";
    pub const TESTNET: &str = "testnet";
    pub const DEVNET: &str = "devnet";
    pub const MAINNET_BETA: &str = "mainnet-beta";
}

/// Protocol version constant
pub const DEFAULT_PROTOCOL_VERSION: &str = "1.18.0";

/// Default capabilities for Solana nodes
pub const DEFAULT_CAPABILITIES: &[&str] = &["gossip", "handshake"];

/// Comprehensive error types for Solana node operations
#[derive(Error, Debug)]
pub enum SolanaNodeError {
    #[error("Network connection failed: {0}")]
    NetworkError(#[from] std::io::Error),

    #[error("Handshake protocol error: {message}")]
    HandshakeError { message: String },

    #[error("Invalid peer data received: {reason}")]
    InvalidPeerData { reason: String },

    #[error("Serialization failed: {0}")]
    SerializationFailed(String),

    #[error("Peer verification failed: {peer_addr}")]
    PeerVerificationFailed { peer_addr: SocketAddr },

    #[error("Protocol version mismatch. Expected: {expected}, Got: {received}")]
    ProtocolVersionMismatch { expected: String, received: String },

    #[error("Timeout occurred during operation")]
    TimeoutError,

    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

/// Outgoing handshake request data structure
#[derive(Debug, Clone, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct HandshakeRequest {
    /// Public key of the requesting node
    pub node_pubkey: Pubkey,

    /// Protocol version supported by this node
    pub protocol_version: String,

    /// Timestamp of the request
    pub timestamp: u64,

    /// Network ID (mainnet-beta, testnet, devnet, localnet)
    pub network_id: String,

    /// Additional node capabilities and features
    pub capabilities: Vec<String>,
}

impl HandshakeRequest {
    /// Create a new handshake request with default values for local development
    pub fn new_local(node_pubkey: Pubkey) -> Self {
        Self {
            node_pubkey,
            protocol_version: DEFAULT_PROTOCOL_VERSION.to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            network_id: network::LOCALNET.to_string(),
            capabilities: DEFAULT_CAPABILITIES.iter().map(|s| s.to_string()).collect(),
        }
    }

    /// Create a new handshake request for testnet
    pub fn new_testnet(node_pubkey: Pubkey) -> Self {
        let mut request = Self::new_local(node_pubkey);
        request.network_id = network::TESTNET.to_string();
        request
    }

    /// Create a new handshake request for the mainnet-beta
    pub fn new_mainnet(node_pubkey: Pubkey) -> Self {
        let mut request = Self::new_local(node_pubkey);
        request.network_id = network::MAINNET_BETA.to_string();
        request
    }
}

/// Incoming handshake response data structure
#[derive(Debug, Clone, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct HandshakeResponse {
    /// Public key of the responding peer
    pub peer_pubkey: Pubkey,

    /// Protocol version of the peer
    pub protocol_version: String,

    /// Network ID of the peer
    pub network_id: String,

    /// Peer's advertised socket address for gossip
    pub gossip_addr: SocketAddr,

    /// Peer capabilities and supported features
    pub capabilities: Vec<String>,

    /// Response timestamp
    pub timestamp: u64,

    /// Success status of the handshake
    pub success: bool,

    /// Optional error message if handshake failed
    pub error_message: Option<String>,
}

impl HandshakeResponse {
    /// Validate the handshake response against expected criteria
    pub fn verify(&self, expected_network: &str) -> Result<(), SolanaNodeError> {
        if !self.success {
            return Err(SolanaNodeError::HandshakeError {
                message: self
                    .error_message
                    .clone()
                    .unwrap_or_else(|| "Unknown handshake failure".to_string()),
            });
        }

        if self.network_id != expected_network {
            return Err(SolanaNodeError::ProtocolVersionMismatch {
                expected: expected_network.to_string(),
                received: self.network_id.clone(),
            });
        }

        // Verify the timestamp is recent (within 5 minutes)
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        if current_time.saturating_sub(self.timestamp) > 300 {
            return Err(SolanaNodeError::InvalidPeerData {
                reason: "Response timestamp too old".to_string(),
            });
        }

        tracing::info!(
            peer_pubkey = %self.peer_pubkey,
            protocol_version = %self.protocol_version,
            network_id = %self.network_id,
            "Handshake response validated successfully"
        );

        Ok(())
    }
}

/// Information about a connected gossip node peer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GossipNodeInfo {
    /// Peer's public key identifier
    pub peer_pubkey: Pubkey,

    /// Network address for gossip communication
    pub gossip_addr: SocketAddr,

    /// Protocol version supported by the peer
    pub protocol_version: String,

    /// Network identifier (mainnet-beta, testnet, etc.)
    pub network_id: String,

    /// Peer capabilities and features
    pub capabilities: Vec<String>,

    /// When this peer was last seen/verified
    pub last_seen: u64,

    /// Connection status with this peer
    pub connection_status: ConnectionStatus,
}

impl GossipNodeInfo {
    /// Create new peer info from a successful handshake response
    pub fn from_handshake_response(response: HandshakeResponse) -> Self {
        Self {
            peer_pubkey: response.peer_pubkey,
            gossip_addr: response.gossip_addr,
            protocol_version: response.protocol_version,
            network_id: response.network_id,
            capabilities: response.capabilities,
            last_seen: response.timestamp,
            connection_status: ConnectionStatus::Connected,
        }
    }
}

/// Status of connection with a peer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConnectionStatus {
    /// Successfully connected and verified
    Connected,
    /// Connection in progress
    Connecting,
    /// Connection failed or lost
    Disconnected,
    /// Peer failed verification
    VerificationFailed,
}

/// Result type for handshake operations
pub type HandshakeResult<T> = Result<T, SolanaNodeError>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn create_test_pubkey() -> Pubkey {
        Pubkey::new_unique()
    }

    #[test]
    fn test_handshake_request_new_local() {
        let pubkey = create_test_pubkey();
        let request = HandshakeRequest::new_local(pubkey);

        assert_eq!(request.node_pubkey, pubkey);
        assert_eq!(request.protocol_version, "1.18.0");
        assert_eq!(request.network_id, "localnet");
        assert!(request.capabilities.contains(&"gossip".to_string()));
        assert!(request.capabilities.contains(&"handshake".to_string()));

        // Verify a timestamp is recent (within last minute)
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert!(current_time.saturating_sub(request.timestamp) < 60);
    }

    #[test]
    fn test_handshake_request_new_testnet() {
        let pubkey = create_test_pubkey();
        let request = HandshakeRequest::new_testnet(pubkey);

        assert_eq!(request.node_pubkey, pubkey);
        assert_eq!(request.network_id, "testnet");
        assert_eq!(request.protocol_version, "1.18.0");
    }

    #[test]
    fn test_handshake_request_new_mainnet() {
        let pubkey = create_test_pubkey();
        let request = HandshakeRequest::new_mainnet(pubkey);

        assert_eq!(request.node_pubkey, pubkey);
        assert_eq!(request.network_id, "mainnet-beta");
        assert_eq!(request.protocol_version, "1.18.0");
    }

    #[test]
    fn test_handshake_request_serialization() {
        let pubkey = create_test_pubkey();
        let request = HandshakeRequest::new_local(pubkey);

        // Test Borsh serialization
        let serialized = borsh::to_vec(&request).unwrap();
        let deserialized: HandshakeRequest = HandshakeRequest::try_from_slice(&serialized).unwrap();

        assert_eq!(request.node_pubkey, deserialized.node_pubkey);
        assert_eq!(request.protocol_version, deserialized.protocol_version);
        assert_eq!(request.network_id, deserialized.network_id);
        assert_eq!(request.capabilities, deserialized.capabilities);
        assert_eq!(request.timestamp, deserialized.timestamp);

        // Test Serde serialization
        let json = serde_json::to_string(&request).unwrap();
        let from_json: HandshakeRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(request.node_pubkey, from_json.node_pubkey);
    }

    #[test]
    fn test_handshake_response_verify_success() {
        let response = HandshakeResponse {
            peer_pubkey: create_test_pubkey(),
            protocol_version: "1.18.0".to_string(),
            network_id: "localnet".to_string(),
            gossip_addr: "127.0.0.1:8001".parse().unwrap(),
            capabilities: vec!["gossip".to_string()],
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            success: true,
            error_message: None,
        };

        assert!(response.verify("localnet").is_ok());
    }

    #[test]
    fn test_handshake_response_verify_failure() {
        let response = HandshakeResponse {
            peer_pubkey: create_test_pubkey(),
            protocol_version: "1.18.0".to_string(),
            network_id: "localnet".to_string(),
            gossip_addr: "127.0.0.1:8001".parse().unwrap(),
            capabilities: vec!["gossip".to_string()],
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            success: false,
            error_message: Some("Test error".to_string()),
        };

        let result = response.verify("localnet");
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(SolanaNodeError::HandshakeError { .. })
        ));
    }

    #[test]
    fn test_handshake_response_verify_network_mismatch() {
        let response = HandshakeResponse {
            peer_pubkey: create_test_pubkey(),
            protocol_version: "1.18.0".to_string(),
            network_id: "testnet".to_string(),
            gossip_addr: "127.0.0.1:8001".parse().unwrap(),
            capabilities: vec!["gossip".to_string()],
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            success: true,
            error_message: None,
        };

        let result = response.verify("localnet");
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(SolanaNodeError::ProtocolVersionMismatch { .. })
        ));
    }

    #[test]
    fn test_handshake_response_verify_old_timestamp() {
        let old_timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            - 400; // 400 seconds ago (over 5 minutes)

        let response = HandshakeResponse {
            peer_pubkey: create_test_pubkey(),
            protocol_version: "1.18.0".to_string(),
            network_id: "localnet".to_string(),
            gossip_addr: "127.0.0.1:8001".parse().unwrap(),
            capabilities: vec!["gossip".to_string()],
            timestamp: old_timestamp,
            success: true,
            error_message: None,
        };

        let result = response.verify("localnet");
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(SolanaNodeError::InvalidPeerData { .. })
        ));
    }

    #[test]
    fn test_gossip_node_info_from_handshake_response() {
        let response = HandshakeResponse {
            peer_pubkey: create_test_pubkey(),
            protocol_version: "1.18.0".to_string(),
            network_id: "localnet".to_string(),
            gossip_addr: "127.0.0.1:8001".parse().unwrap(),
            capabilities: vec!["gossip".to_string(), "handshake".to_string()],
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            success: true,
            error_message: None,
        };

        let peer_info = GossipNodeInfo::from_handshake_response(response.clone());

        assert_eq!(peer_info.peer_pubkey, response.peer_pubkey);
        assert_eq!(peer_info.gossip_addr, response.gossip_addr);
        assert_eq!(peer_info.protocol_version, response.protocol_version);
        assert_eq!(peer_info.network_id, response.network_id);
        assert_eq!(peer_info.capabilities, response.capabilities);
        assert_eq!(peer_info.last_seen, response.timestamp);
        assert!(matches!(
            peer_info.connection_status,
            ConnectionStatus::Connected
        ));
    }

    #[test]
    fn test_connection_status_variants() {
        let connected = ConnectionStatus::Connected;
        let connecting = ConnectionStatus::Connecting;
        let disconnected = ConnectionStatus::Disconnected;
        let verification_failed = ConnectionStatus::VerificationFailed;

        // Test serialization
        let connected_json = serde_json::to_string(&connected).unwrap();
        let connected_back: ConnectionStatus = serde_json::from_str(&connected_json).unwrap();
        assert!(matches!(connected_back, ConnectionStatus::Connected));

        // Ensure all variants exist
        assert!(matches!(connecting, ConnectionStatus::Connecting));
        assert!(matches!(disconnected, ConnectionStatus::Disconnected));
        assert!(matches!(
            verification_failed,
            ConnectionStatus::VerificationFailed
        ));
    }

    #[test]
    fn test_error_display_formatting() {
        let network_error = SolanaNodeError::NetworkError(std::io::Error::new(
            std::io::ErrorKind::ConnectionRefused,
            "Connection refused",
        ));
        let error_str = format!("{}", network_error);
        assert!(error_str.contains("Network connection failed"));

        let handshake_error = SolanaNodeError::HandshakeError {
            message: "Invalid signature".to_string(),
        };
        let error_str = format!("{}", handshake_error);
        assert!(error_str.contains("Invalid signature"));

        let protocol_mismatch = SolanaNodeError::ProtocolVersionMismatch {
            expected: "localnet".to_string(),
            received: "testnet".to_string(),
        };
        let error_str = format!("{}", protocol_mismatch);
        assert!(error_str.contains("localnet"));
        assert!(error_str.contains("testnet"));
    }
}
