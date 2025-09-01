//! Solana network protocol implementations
//!
//! This module contains all Solana-specific networking functionality including
//! - Gossip node management for P2P communication
//! - Handshake client for establishing secure peer connections
//! - Domain models for network messages and data structures

/// Core gossip node implementation for UDP-based P2P networking
pub mod gossip_node;

/// Handshake protocol client for peer authentication and connection establishment
pub mod handshake_client;

/// Domain models, data structures, and serialization for network communication
pub mod domain;

// Re-export public types for easier access
pub use gossip_node::GossipNode;
pub use domain::GossipNodeInfo;
pub use handshake_client::{HandshakeClient, HandshakeResult};
pub use domain::{HandshakeRequest, HandshakeResponse, SolanaNodeError};