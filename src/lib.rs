//! # P2P Solana Handshake
//!
//! A Rust library for implementing Solana network handshake protocols.
//! This library provides functionality to connect to Solana gossip nodes
//! and perform protocol handshakes for P2P communication.

/// CLI argument parsing and configuration management
pub mod arguments_parser;

/// Structured logging and telemetry infrastructure
pub mod telemetry;

/// Solana-specific networking and protocol implementations
pub mod solana {
    /// Core gossip node implementation for Solana P2P networking
    pub mod gossip_node;

    /// Handshake client for establishing secure connections with peers
    pub mod handshake_client;

    /// Domain models and data structures for Solana network communication
    pub mod domain;

    pub use domain::{network, GossipNodeInfo, DEFAULT_CAPABILITIES, DEFAULT_PROTOCOL_VERSION};
    pub use domain::{HandshakeRequest, HandshakeResponse, SolanaNodeError};
    /// Module exports for public API
    pub use gossip_node::GossipNode;
    pub use handshake_client::{HandshakeClient, HandshakeResult};
}

// Re-export commonly used types for easier access
pub use arguments_parser::Args;
pub use solana::{GossipNode, HandshakeClient, HandshakeRequest, HandshakeResponse};
pub use telemetry::setup_telemetry;
