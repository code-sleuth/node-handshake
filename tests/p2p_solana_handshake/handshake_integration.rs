use p2p_solana_handshake::solana::{
    domain::{network, ConnectionStatus, GossipNodeInfo},
    GossipNode, HandshakeClient, SolanaNodeError,
};
use solana_sdk::pubkey::Pubkey;
use std::time::Duration;
use tokio::time::sleep;

/// Helper function to find an available port
async fn find_available_port() -> u16 {
    use tokio::net::UdpSocket;

    let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    socket.local_addr().unwrap().port()
}

/// Helper function to create a test gossip node
async fn create_test_gossip_node(network_id: &str) -> (GossipNode, u16) {
    let port = find_available_port().await;
    let bind_addr = format!("127.0.0.1:{}", port);
    let node_pubkey = Pubkey::new_unique();

    let gossip_node = GossipNode::new(&bind_addr, node_pubkey, network_id.to_string())
        .await
        .unwrap();

    (gossip_node, port)
}

/// Helper function to create a test handshake client
async fn create_test_handshake_client(network_id: &str) -> HandshakeClient {
    let node_pubkey = Pubkey::new_unique();

    HandshakeClient::with_config(
        "127.0.0.1:0",
        node_pubkey,
        network_id.to_string(),
        Duration::from_secs(5),
        2,
    )
    .await
    .unwrap()
}

#[tokio::test]
async fn test_successful_handshake_localnet() {
    // Create a gossip node (server)
    let (mut gossip_node, server_port) = create_test_gossip_node("localnet").await;

    // Start the gossip node in the background
    let server_handle = tokio::spawn(async move {
        // Run for a short time to handle the handshake
        tokio::select! {
            _ = gossip_node.run() => {},
            _ = sleep(Duration::from_secs(10)) => {} // Timeout after 10 seconds
        }
    });

    // Give the server time to start
    sleep(Duration::from_millis(100)).await;

    // Create a handshake client
    let client = create_test_handshake_client(network::LOCALNET).await;

    // Perform handshake
    let server_addr = format!("127.0.0.1:{}", server_port).parse().unwrap();
    let result = client.perform_handshake(server_addr).await;

    // Verify a successful handshake
    assert!(result.is_ok(), "Handshake should succeed: {:?}", result);

    let peer_info = result.unwrap();
    assert_eq!(peer_info.network_id, network::LOCALNET);
    assert_eq!(peer_info.gossip_addr, server_addr);

    // Clean up
    server_handle.abort();
}

#[tokio::test]
async fn test_handshake_network_mismatch() {
    // Create a gossip node on a testnet
    let (mut gossip_node, server_port) = create_test_gossip_node(network::TESTNET).await;

    // Start the gossip node in the background
    let server_handle = tokio::spawn(async move {
        tokio::select! {
            _ = gossip_node.run() => {},
            _ = sleep(Duration::from_secs(10)) => {}
        }
    });

    // Give the server time to start
    sleep(Duration::from_millis(100)).await;

    // Create a client configured for localnet (mismatch)
    let client = create_test_handshake_client(network::LOCALNET).await;

    // Perform handshake
    let server_addr = format!("127.0.0.1:{}", server_port).parse().unwrap();
    let result = client.perform_handshake(server_addr).await;

    // Verify handshake fails due to network mismatch
    assert!(
        result.is_err(),
        "Handshake should fail due to network mismatch"
    );

    match result.unwrap_err() {
        SolanaNodeError::HandshakeError { message } => {
            assert!(message.contains("Network ID mismatch"));
            assert!(message.contains(network::TESTNET));
            assert!(message.contains(network::LOCALNET));
        }
        other => panic!(
            "Expected HandshakeError with network mismatch, got: {:?}",
            other
        ),
    }

    // Clean up
    server_handle.abort();
}

#[tokio::test]
async fn test_handshake_timeout() {
    // Create a handshake client with a very short timeout
    let node_pubkey = Pubkey::new_unique();
    let client = HandshakeClient::with_config(
        "127.0.0.1:0",
        node_pubkey,
        network::LOCALNET.to_string(),
        Duration::from_millis(100), // Very short timeout
        1,                          // Only 1 retry
    )
    .await
    .unwrap();

    // Try to connect to a non-existent server
    let non_existent_addr = "127.0.0.1:9999".parse().unwrap();
    let result = client.perform_handshake(non_existent_addr).await;

    // Verify timeout error
    assert!(result.is_err(), "Handshake should timeout");
    assert!(matches!(result.unwrap_err(), SolanaNodeError::TimeoutError));
}

#[tokio::test]
async fn test_handshake_multiple_retries() {
    let node_pubkey = Pubkey::new_unique();
    let client = HandshakeClient::with_config(
        "127.0.0.1:0",
        node_pubkey,
        network::LOCALNET.to_string(),
        Duration::from_millis(500),
        3, // 3 retries
    )
    .await
    .unwrap();

    let start_time = std::time::Instant::now();

    // Try to connect to a non-existent server
    let non_existent_addr = "127.0.0.1:9999".parse().unwrap();
    let result = client.perform_handshake(non_existent_addr).await;

    let duration = start_time.elapsed();

    // Should fail after all retries
    assert!(result.is_err());

    // Should have taken some time due to retries with exponential backoff
    // 3 attempts with exponential backoff: 0s + 1s + 2s = at least 3 seconds
    assert!(
        duration >= Duration::from_secs(2),
        "Should take time for retries"
    );
}

#[tokio::test]
async fn test_batch_handshakes() {
    // Create multiple gossip nodes
    let (mut node1, port1) = create_test_gossip_node(network::LOCALNET).await;
    let (mut node2, port2) = create_test_gossip_node(network::LOCALNET).await;

    // Start both nodes
    let handle1 = tokio::spawn(async move {
        tokio::select! {
            _ = node1.run() => {},
            _ = sleep(Duration::from_secs(10)) => {}
        }
    });

    let handle2 = tokio::spawn(async move {
        tokio::select! {
            _ = node2.run() => {},
            _ = sleep(Duration::from_secs(10)) => {}
        }
    });

    // Give servers time to start
    sleep(Duration::from_millis(100)).await;

    // Create client
    let client = create_test_handshake_client(network::LOCALNET).await;

    // Perform batch handshakes
    let peer_addrs = vec![
        format!("127.0.0.1:{}", port1).parse().unwrap(),
        format!("127.0.0.1:{}", port2).parse().unwrap(),
    ];

    let results = client.perform_batch_handshakes(peer_addrs.clone()).await;

    // Verify results
    assert_eq!(results.len(), 2);

    let successful_count = results.iter().filter(|(_, result)| result.is_ok()).count();
    assert_eq!(successful_count, 2, "Both handshakes should succeed");

    // Verify each result
    for (addr, result) in results {
        assert!(peer_addrs.contains(&addr));
        assert!(result.is_ok(), "Handshake with {:?} should succeed", addr);

        let peer_info = result.unwrap();
        assert_eq!(peer_info.network_id, network::LOCALNET);
        assert_eq!(peer_info.gossip_addr, addr);
    }

    // Clean up
    handle1.abort();
    handle2.abort();
}

#[tokio::test]
async fn test_batch_handshakes_mixed_results() {
    // Create one working gossip node
    let (mut gossip_node, working_port) = create_test_gossip_node(network::LOCALNET).await;

    // Start the working node
    let handle = tokio::spawn(async move {
        tokio::select! {
            _ = gossip_node.run() => {},
            _ = sleep(Duration::from_secs(10)) => {}
        }
    });

    // Give server time to start
    sleep(Duration::from_millis(100)).await;

    // Create client with short timeout for faster test
    let node_pubkey = Pubkey::new_unique();
    let client = HandshakeClient::with_config(
        "127.0.0.1:0",
        node_pubkey,
        network::LOCALNET.to_string(),
        Duration::from_millis(500),
        1,
    )
    .await
    .unwrap();

    // Try handshakes with one working and one non-existent server
    let peer_addrs = vec![
        format!("127.0.0.1:{}", working_port).parse().unwrap(), // Should work
        "127.0.0.1:9999".parse().unwrap(),                      // Should fail
    ];

    let results = client.perform_batch_handshakes(peer_addrs).await;

    // Verify mixed results
    assert_eq!(results.len(), 2);

    let successful_count = results.iter().filter(|(_, result)| result.is_ok()).count();
    let failed_count = results.iter().filter(|(_, result)| result.is_err()).count();

    assert_eq!(successful_count, 1, "One handshake should succeed");
    assert_eq!(failed_count, 1, "One handshake should fail");

    // Clean up
    handle.abort();
}

#[tokio::test]
async fn test_gossip_node_peer_registry() {
    let (mut gossip_node, _) = create_test_gossip_node(network::LOCALNET).await;

    // Initially no peers
    assert_eq!(gossip_node.get_peers().len(), 0);

    // Add a peer manually (simulating a successful handshake)
    let peer_info = GossipNodeInfo {
        peer_pubkey: Pubkey::new_unique(),
        gossip_addr: "127.0.0.1:8001".parse().unwrap(),
        protocol_version: "1.18.0".to_string(),
        network_id: network::LOCALNET.to_string(),
        capabilities: vec!["gossip".to_string()],
        last_seen: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        connection_status: ConnectionStatus::Connected,
    };

    let peer_addr = peer_info.gossip_addr;
    gossip_node.register_peer(peer_info.clone()).await;

    // Verify peer is registered
    assert_eq!(gossip_node.get_peers().len(), 1);

    let registered_peer = gossip_node.get_peer_info(&peer_addr).unwrap();
    assert_eq!(registered_peer.peer_pubkey, peer_info.peer_pubkey);
    assert_eq!(registered_peer.gossip_addr, peer_info.gossip_addr);
    assert_eq!(registered_peer.network_id, peer_info.network_id);
}

#[tokio::test]
async fn test_different_network_types() {
    for network_id in [network::LOCALNET, network::TESTNET] {
        // Create a gossip node and client for the same network
        let (mut gossip_node, server_port) = create_test_gossip_node(network_id).await;

        let server_handle = tokio::spawn(async move {
            tokio::select! {
                _ = gossip_node.run() => {},
                _ = sleep(Duration::from_secs(10)) => {}
            }
        });

        sleep(Duration::from_millis(100)).await;

        let client = create_test_handshake_client(network_id).await;
        let server_addr = format!("127.0.0.1:{}", server_port).parse().unwrap();

        let result = client.perform_handshake(server_addr).await;

        assert!(
            result.is_ok(),
            "Handshake should succeed for network: {}, error: {:?}",
            network_id,
            result
        );

        let peer_info = result.unwrap();
        assert_eq!(peer_info.network_id, network_id);

        server_handle.abort();
        sleep(Duration::from_millis(100)).await; // Small delay between tests
    }
}
