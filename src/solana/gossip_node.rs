//! Gossip node implementation for Solana P2P networking
//!
//! This module provides the core gossip node functionality for participating
//! in the Solana network's peer-to-peer communication layer.

use crate::solana::domain::{
    ConnectionStatus, GossipNodeInfo, HandshakeRequest, HandshakeResponse, SolanaNodeError,
};
use borsh::BorshDeserialize;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use std::net::SocketAddr;
use tokio::net::UdpSocket;
use tokio::time::{timeout, Duration};
use tracing::{debug, error, info, warn};

/// Maximum UDP packet size for Solana gossip protocol (1280 bytes)
const MAX_GOSSIP_PACKET_SIZE: usize = 1280;

/// Default timeout for network operations (30 seconds)
const DEFAULT_NETWORK_TIMEOUT: Duration = Duration::from_secs(30);

/// Core gossip node for Solana P2P networking
pub struct GossipNode {
    /// UDP socket for network communication
    socket: UdpSocket,

    /// This node's public key identifier
    node_pubkey: Pubkey,

    /// Network ID this node operates on
    network_id: String,

    /// Registry of known peers and their connection status
    peer_registry: HashMap<SocketAddr, GossipNodeInfo>,
    // Note: For future extension, message routing channels can be added here
    // Currently using direct synchronous handling for simplicity
}

impl GossipNode {
    /// Create a new gossip node instance
    #[tracing::instrument(skip(bind_addr))]
    pub async fn new(
        bind_addr: &str,
        node_pubkey: Pubkey,
        network_id: String,
    ) -> Result<Self, SolanaNodeError> {
        info!(
            bind_addr = %bind_addr,
            node_pubkey = %node_pubkey,
            network_id = %network_id,
            "Initializing gossip node"
        );

        let socket = UdpSocket::bind(bind_addr).await.map_err(|e| {
            error!(error = %e, bind_addr = %bind_addr, "Failed to bind UDP socket");
            SolanaNodeError::NetworkError(e)
        })?;

        let local_addr = socket.local_addr().map_err(SolanaNodeError::NetworkError)?;

        info!(local_addr = %local_addr, "UDP socket bound successfully");

        // Channels removed for simplicity - using direct synchronous handling

        Ok(Self {
            socket,
            node_pubkey,
            network_id,
            peer_registry: HashMap::new(),
        })
    }

    /// Get the local address this node is bound to
    pub fn local_addr(&self) -> Result<SocketAddr, SolanaNodeError> {
        self.socket
            .local_addr()
            .map_err(SolanaNodeError::NetworkError)
    }

    /// Add a peer to the registry or update existing peer info
    #[tracing::instrument(skip(self))]
    pub async fn register_peer(&mut self, peer_info: GossipNodeInfo) {
        debug!(
            peer_addr = %peer_info.gossip_addr,
            peer_pubkey = %peer_info.peer_pubkey,
            "Registering peer in gossip node"
        );

        self.peer_registry.insert(peer_info.gossip_addr, peer_info);

        info!(
            total_peers = self.peer_registry.len(),
            "Peer registry updated"
        );
    }

    /// Get information about all registered peers
    pub fn get_peers(&self) -> Vec<&GossipNodeInfo> {
        self.peer_registry.values().collect()
    }

    /// Get information about a specific peer
    pub fn get_peer_info(&self, addr: &SocketAddr) -> Option<&GossipNodeInfo> {
        self.peer_registry.get(addr)
    }

    /// Send a handshake request to a peer
    #[tracing::instrument(skip(self, request))]
    pub async fn send_handshake_request(
        &self,
        peer_addr: SocketAddr,
        request: HandshakeRequest,
    ) -> Result<(), SolanaNodeError> {
        debug!(
            peer_addr = %peer_addr,
            protocol_version = %request.protocol_version,
            "Sending handshake request"
        );

        let serialized_request = borsh::to_vec(&request)
            .map_err(|e| SolanaNodeError::SerializationFailed(e.to_string()))?;

        if serialized_request.len() > MAX_GOSSIP_PACKET_SIZE {
            return Err(SolanaNodeError::InvalidPeerData {
                reason: format!("Request too large: {} bytes", serialized_request.len()),
            });
        }

        let result = timeout(
            DEFAULT_NETWORK_TIMEOUT,
            self.socket.send_to(&serialized_request, peer_addr),
        )
        .await;

        match result {
            Ok(Ok(bytes_sent)) => {
                info!(
                    peer_addr = %peer_addr,
                    bytes_sent = bytes_sent,
                    "Handshake request sent successfully"
                );
                Ok(())
            }
            Ok(Err(e)) => {
                error!(
                    error = %e,
                    peer_addr = %peer_addr,
                    "Failed to send handshake request"
                );
                Err(SolanaNodeError::NetworkError(e))
            }
            Err(_) => {
                warn!(
                    peer_addr = %peer_addr,
                    timeout_secs = DEFAULT_NETWORK_TIMEOUT.as_secs(),
                    "Handshake request timed out"
                );
                Err(SolanaNodeError::TimeoutError)
            }
        }
    }

    /// Start the main event loop for processing network messages
    #[tracing::instrument(skip(self))]
    pub async fn run(&mut self) -> Result<(), SolanaNodeError> {
        info!("Starting gossip node main event loop");

        let mut packet_buffer = vec![0u8; MAX_GOSSIP_PACKET_SIZE];

        loop {
            // Handle incoming UDP packets
            match self.socket.recv_from(&mut packet_buffer).await {
                Ok((packet_length, sender_addr)) => {
                    debug!(
                        sender_addr = %sender_addr,
                        packet_length = packet_length,
                        "Received UDP packet"
                    );

                    if let Err(e) = self
                        .process_incoming_packet(&packet_buffer[..packet_length], sender_addr)
                        .await
                    {
                        warn!(
                            error = %e,
                            sender_addr = %sender_addr,
                            "Failed to process incoming packet"
                        );
                    }
                }
                Err(e) => {
                    error!(error = %e, "UDP receive error");
                    return Err(SolanaNodeError::NetworkError(e));
                }
            }
        }
    }

    /// Process an incoming network packet
    #[tracing::instrument(skip(self, packet_data))]
    async fn process_incoming_packet(
        &mut self,
        packet_data: &[u8],
        sender_addr: SocketAddr,
    ) -> Result<(), SolanaNodeError> {
        // Try to deserialize as a handshake request first
        if let Ok(handshake_request) = HandshakeRequest::try_from_slice(packet_data) {
            info!(
                sender_addr = %sender_addr,
                peer_pubkey = %handshake_request.node_pubkey,
                protocol_version = %handshake_request.protocol_version,
                "Received handshake request"
            );

            return self
                .handle_handshake_request(sender_addr, handshake_request)
                .await;
        }

        // Try to deserialize as a handshake response
        if let Ok(handshake_response) = HandshakeResponse::try_from_slice(packet_data) {
            info!(
                sender_addr = %sender_addr,
                peer_pubkey = %handshake_response.peer_pubkey,
                success = handshake_response.success,
                "Received handshake response"
            );

            return self
                .handle_handshake_response(sender_addr, handshake_response)
                .await;
        }

        debug!(
            sender_addr = %sender_addr,
            packet_size = packet_data.len(),
            "Received unknown packet type"
        );

        Ok(())
    }

    /// Handle an incoming handshake request from a peer
    #[tracing::instrument(skip(self, request))]
    async fn handle_handshake_request(
        &mut self,
        sender_addr: SocketAddr,
        request: HandshakeRequest,
    ) -> Result<(), SolanaNodeError> {
        // Validate network compatibility
        if request.network_id != self.network_id {
            let error_response = HandshakeResponse {
                peer_pubkey: self.node_pubkey,
                protocol_version: "1.18.0".to_string(),
                network_id: self.network_id.clone(),
                gossip_addr: self.local_addr()?,
                capabilities: vec!["gossip".to_string()],
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                success: false,
                error_message: Some(format!(
                    "Network ID mismatch: expected {}, got {}",
                    self.network_id, request.network_id
                )),
            };

            return self
                .send_handshake_response(sender_addr, error_response)
                .await;
        }

        // Create a successful handshake response
        let response = HandshakeResponse {
            peer_pubkey: self.node_pubkey,
            protocol_version: request.protocol_version.clone(),
            network_id: self.network_id.clone(),
            gossip_addr: self.local_addr()?,
            capabilities: vec!["gossip".to_string(), "handshake".to_string()],
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            success: true,
            error_message: None,
        };

        // Register the peer
        let peer_info = GossipNodeInfo {
            peer_pubkey: request.node_pubkey,
            gossip_addr: sender_addr,
            protocol_version: request.protocol_version,
            network_id: request.network_id,
            capabilities: request.capabilities,
            last_seen: request.timestamp,
            connection_status: ConnectionStatus::Connected,
        };

        self.register_peer(peer_info).await;

        // Send response
        self.send_handshake_response(sender_addr, response).await
    }

    /// Handle an incoming handshake response from a peer
    #[tracing::instrument(skip(self, response))]
    async fn handle_handshake_response(
        &mut self,
        sender_addr: SocketAddr,
        response: HandshakeResponse,
    ) -> Result<(), SolanaNodeError> {
        if let Err(e) = response.verify(&self.network_id) {
            warn!(
                error = %e,
                sender_addr = %sender_addr,
                "Handshake response verification failed"
            );
            return Err(e);
        }

        // Register the peer from a successful handshake
        let peer_info = GossipNodeInfo::from_handshake_response(response);
        self.register_peer(peer_info).await;

        Ok(())
    }

    /// Send a handshake response to a peer
    #[tracing::instrument(skip(self, response))]
    async fn send_handshake_response(
        &self,
        peer_addr: SocketAddr,
        response: HandshakeResponse,
    ) -> Result<(), SolanaNodeError> {
        let serialized_response = borsh::to_vec(&response)
            .map_err(|e| SolanaNodeError::SerializationFailed(e.to_string()))?;

        let result = timeout(
            DEFAULT_NETWORK_TIMEOUT,
            self.socket.send_to(&serialized_response, peer_addr),
        )
        .await;

        match result {
            Ok(Ok(bytes_sent)) => {
                debug!(
                    peer_addr = %peer_addr,
                    bytes_sent = bytes_sent,
                    success = response.success,
                    "Handshake response sent"
                );
                Ok(())
            }
            Ok(Err(e)) => Err(SolanaNodeError::NetworkError(e)),
            Err(_) => Err(SolanaNodeError::TimeoutError),
        }
    }
}

// Re-export for convenience
