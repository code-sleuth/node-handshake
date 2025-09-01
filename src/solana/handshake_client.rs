//! Handshake client for establishing connections with Solana gossip nodes
//!
//! This module provides high-level client functionality for performing
//! handshake protocols with remote Solana nodes.

use crate::solana::domain::{GossipNodeInfo, HandshakeRequest, HandshakeResponse, SolanaNodeError};
use borsh::BorshDeserialize;
use solana_sdk::pubkey::Pubkey;
use std::net::SocketAddr;
use tokio::net::UdpSocket;
use tokio::time::{timeout, Duration};
use tracing::{debug, error, info, instrument, warn};

/// Default timeout for handshake operations (15 seconds)
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(15);

/// Maximum number of handshake retry attempts
const MAX_HANDSHAKE_RETRIES: u8 = 3;

/// Result type for handshake operations
pub type HandshakeResult<T> = Result<T, SolanaNodeError>;

/// High-level client for performing Solana node handshakes
#[derive(Debug)]
pub struct HandshakeClient {
    /// UDP socket for communication
    socket: UdpSocket,

    /// This client's node public key
    node_pubkey: Pubkey,

    /// Network ID to operate on
    network_id: String,

    /// Timeout duration for operations
    operation_timeout: Duration,

    /// Maximum retry attempts
    max_retries: u8,
}

impl HandshakeClient {
    /// Create a new handshake client
    #[instrument(skip(bind_addr))]
    pub async fn new(
        bind_addr: &str,
        node_pubkey: Pubkey,
        network_id: String,
    ) -> HandshakeResult<Self> {
        info!(
            bind_addr = %bind_addr,
            node_pubkey = %node_pubkey,
            network_id = %network_id,
            "Creating handshake client"
        );

        let socket = UdpSocket::bind(bind_addr).await.map_err(|e| {
            error!(error = %e, bind_addr = %bind_addr, "Failed to bind handshake client socket");
            SolanaNodeError::NetworkError(e)
        })?;

        let local_addr = socket.local_addr().map_err(SolanaNodeError::NetworkError)?;

        info!(local_addr = %local_addr, "Handshake client socket bound");

        Ok(Self {
            socket,
            node_pubkey,
            network_id,
            operation_timeout: HANDSHAKE_TIMEOUT,
            max_retries: MAX_HANDSHAKE_RETRIES,
        })
    }

    /// Create a handshake client with custom timeout and retry settings
    pub async fn with_config(
        bind_addr: &str,
        node_pubkey: Pubkey,
        network_id: String,
        operation_timeout: Duration,
        max_retries: u8,
    ) -> HandshakeResult<Self> {
        let mut client = Self::new(bind_addr, node_pubkey, network_id).await?;
        client.operation_timeout = operation_timeout;
        client.max_retries = max_retries;
        Ok(client)
    }

    /// Perform a handshake with a remote Solana node
    #[instrument(skip(self))]
    pub async fn perform_handshake(
        &self,
        peer_addr: SocketAddr,
    ) -> HandshakeResult<GossipNodeInfo> {
        info!(peer_addr = %peer_addr, "Starting handshake");

        let mut last_error = None;

        for attempt in 1..=self.max_retries {
            debug!(
                peer_addr = %peer_addr,
                attempt = attempt,
                max_retries = self.max_retries,
                "Handshake attempt"
            );

            match self.attempt_handshake(peer_addr).await {
                Ok(peer_info) => {
                    info!(
                        peer_addr = %peer_addr,
                        peer_pubkey = %peer_info.peer_pubkey,
                        attempt = attempt,
                        "Handshake completed successfully"
                    );
                    return Ok(peer_info);
                }
                Err(e) => {
                    warn!(
                        error = %e,
                        peer_addr = %peer_addr,
                        attempt = attempt,
                        "Handshake attempt failed"
                    );
                    last_error = Some(e);

                    // Don't retry on certain error types
                    if matches!(
                        last_error,
                        Some(SolanaNodeError::ProtocolVersionMismatch { .. })
                    ) {
                        break;
                    }

                    if attempt < self.max_retries {
                        // Exponential backoff: 1s, 2s, 4s, etc.
                        let delay = Duration::from_secs(1 << (attempt - 1));
                        debug!(delay_secs = delay.as_secs(), "Waiting before retry");
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }

        error!(
            peer_addr = %peer_addr,
            max_retries = self.max_retries,
            "Handshake failed after all retry attempts"
        );

        Err(last_error.unwrap_or(SolanaNodeError::HandshakeError {
            message: "Handshake failed after maximum retries".to_string(),
        }))
    }

    /// Attempt a single handshake with a peer
    #[instrument(skip(self))]
    async fn attempt_handshake(&self, peer_addr: SocketAddr) -> HandshakeResult<GossipNodeInfo> {
        // Create a handshake request
        let request = match self.network_id.as_str() {
            "mainnet-beta" => HandshakeRequest::new_mainnet(self.node_pubkey),
            "testnet" => HandshakeRequest::new_testnet(self.node_pubkey),
            _ => HandshakeRequest::new_local(self.node_pubkey),
        };

        // Send handshake request
        self.send_handshake_request(peer_addr, &request).await?;

        // Wait for response
        let response = self.receive_handshake_response().await?;

        // Verify response
        response.verify(&self.network_id)?;

        // Convert to peer info
        let peer_info = GossipNodeInfo::from_handshake_response(response);

        Ok(peer_info)
    }

    /// Send a handshake request to a peer
    #[instrument(skip(self, request))]
    async fn send_handshake_request(
        &self,
        peer_addr: SocketAddr,
        request: &HandshakeRequest,
    ) -> HandshakeResult<()> {
        let serialized_request = borsh::to_vec(request)
            .map_err(|e| SolanaNodeError::SerializationFailed(e.to_string()))?;

        debug!(
            peer_addr = %peer_addr,
            request_size = serialized_request.len(),
            protocol_version = %request.protocol_version,
            "Sending handshake request"
        );

        let result = timeout(
            self.operation_timeout,
            self.socket.send_to(&serialized_request, peer_addr),
        )
        .await;

        match result {
            Ok(Ok(bytes_sent)) => {
                debug!(
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
                    "Network error sending handshake request"
                );
                Err(SolanaNodeError::NetworkError(e))
            }
            Err(_) => {
                warn!(
                    peer_addr = %peer_addr,
                    timeout_secs = self.operation_timeout.as_secs(),
                    "Handshake request send timeout"
                );
                Err(SolanaNodeError::TimeoutError)
            }
        }
    }

    /// Receive a handshake response from any peer
    #[instrument(skip(self))]
    async fn receive_handshake_response(&self) -> HandshakeResult<HandshakeResponse> {
        let mut buffer = vec![0u8; 1280]; // Solana gossip packet size limit

        debug!("Waiting for handshake response");

        let result = timeout(self.operation_timeout, self.socket.recv_from(&mut buffer)).await;

        match result {
            Ok(Ok((bytes_received, sender_addr))) => {
                debug!(
                    sender_addr = %sender_addr,
                    bytes_received = bytes_received,
                    "Received handshake response"
                );

                let response = HandshakeResponse::try_from_slice(&buffer[..bytes_received])
                    .map_err(|e| {
                        warn!(
                            error = %e,
                            sender_addr = %sender_addr,
                            "Failed to deserialize handshake response"
                        );
                        SolanaNodeError::InvalidPeerData {
                            reason: format!("Failed to deserialize response: {}", e),
                        }
                    })?;

                debug!(
                    sender_addr = %sender_addr,
                    peer_pubkey = %response.peer_pubkey,
                    success = response.success,
                    "Handshake response deserialized"
                );

                Ok(response)
            }
            Ok(Err(e)) => {
                error!(error = %e, "Network error receiving handshake response");
                Err(SolanaNodeError::NetworkError(e))
            }
            Err(_) => {
                warn!(
                    timeout_secs = self.operation_timeout.as_secs(),
                    "Handshake response receive timeout"
                );
                Err(SolanaNodeError::TimeoutError)
            }
        }
    }

    /// Perform handshakes with multiple peers concurrently
    #[instrument(skip(self, peer_addrs))]
    pub async fn perform_batch_handshakes(
        &self,
        peer_addrs: Vec<SocketAddr>,
    ) -> Vec<(SocketAddr, HandshakeResult<GossipNodeInfo>)> {
        info!(peer_count = peer_addrs.len(), "Starting batch handshakes");

        // For now, just perform handshakes sequentially to avoid socket sharing issues
        // In a production implementation; you'd want a proper connection pool
        let mut results = Vec::new();

        for peer_addr in peer_addrs {
            let result = self.perform_handshake(peer_addr).await;
            results.push((peer_addr, result));
        }

        let successful_count = results.iter().filter(|(_, result)| result.is_ok()).count();

        info!(
            total_peers = results.len(),
            successful_handshakes = successful_count,
            "Batch handshakes completed"
        );

        results
    }
}
