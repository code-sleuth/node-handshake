use clap::Parser;
use p2p_solana_handshake::{
    arguments_parser::{ApplicationMode, Args},
    solana::{GossipNode, HandshakeClient},
    telemetry::{log_operation_failure, log_operation_success, setup_telemetry, TelemetryConfig},
};
use solana_sdk::pubkey::Pubkey;
use std::net::{SocketAddr, ToSocketAddrs};
use std::time::Instant;
use tracing::{error, info, warn};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Parse command-line arguments
    let args = Args::parse();

    // Validate arguments
    if let Err(e) = args.validate() {
        eprintln!("Argument validation error: {}", e);
        std::process::exit(1);
    }

    // Setup telemetry based on configuration
    let telemetry_config = TelemetryConfig::from_env();
    setup_telemetry(args.log_level.clone(), args.log_format.clone())
        .map_err(|e| anyhow::anyhow!("Failed to setup telemetry: {}", e))?;

    info!(
        service_name = %telemetry_config.service_name,
        service_version = %telemetry_config.service_version,
        environment = %telemetry_config.environment,
        bind_address = %args.bind_address,
        network_id = %args.network_id,
        mode = ?args.mode,
        "Starting Solana P2P handshake application"
    );

    let start_time = Instant::now();

    // Generate a unique node public key for this session
    let node_pubkey = Pubkey::new_unique();
    info!(node_pubkey = %node_pubkey, "Generated node public key");

    let result = match args.mode {
        ApplicationMode::Client => run_client_mode(&args, node_pubkey).await,
        ApplicationMode::Server => run_server_mode(&args, node_pubkey).await,
    };

    let duration = start_time.elapsed();

    match result {
        Ok(_) => {
            log_operation_success("application", duration);
            info!("Application completed successfully");
        }
        Err(e) => {
            log_operation_failure("application", e.as_ref(), duration);
            error!(error = %e, "Application failed");
            std::process::exit(1);
        }
    }

    Ok(())
}

/// Run the application in client mode - perform handshakes with peers
async fn run_client_mode(args: &Args, node_pubkey: Pubkey) -> anyhow::Result<()> {
    info!(
        peer_count = args.peer_addresses.len(),
        continuous = args.continuous,
        "Running in client mode"
    );

    // Create a handshake client
    let handshake_client = HandshakeClient::with_config(
        &args.bind_address,
        node_pubkey,
        args.network_id.to_string(),
        args.network_timeout(),
        args.max_retries,
    )
    .await
    .map_err(|e| anyhow::anyhow!("Failed to create handshake client: {}", e))?;

    if args.continuous {
        run_continuous_client_mode(&handshake_client, args).await
    } else {
        run_single_client_mode(&handshake_client, args).await
    }
}

/// Run client in single-shot mode
async fn run_single_client_mode(client: &HandshakeClient, args: &Args) -> anyhow::Result<()> {
    info!("Performing single batch of handshakes");

    // Resolve hostnames to SocketAddrs
    let peer_addresses = resolve_peer_addresses(&args.peer_addresses).await?;

    let start_time = Instant::now();
    let results = client.perform_batch_handshakes(peer_addresses).await;
    let duration = start_time.elapsed();

    let successful_count = results.iter().filter(|(_, result)| result.is_ok()).count();

    info!(
        total_peers = results.len(),
        successful_handshakes = successful_count,
        duration_ms = duration.as_millis(),
        "Batch handshakes completed"
    );

    // Log individual results
    for (peer_addr, result) in results {
        match result {
            Ok(peer_info) => {
                info!(
                    peer_addr = %peer_addr,
                    peer_pubkey = %peer_info.peer_pubkey,
                    protocol_version = %peer_info.protocol_version,
                    "Handshake successful"
                );
            }
            Err(e) => {
                warn!(
                    peer_addr = %peer_addr,
                    error = %e,
                    "Handshake failed"
                );
            }
        }
    }

    Ok(())
}

/// Run client in continuous mode
async fn run_continuous_client_mode(client: &HandshakeClient, args: &Args) -> anyhow::Result<()> {
    info!(
        interval_secs = args.interval_secs,
        "Running in continuous client mode"
    );

    // Resolve hostnames to SocketAddrs once
    let peer_addresses = resolve_peer_addresses(&args.peer_addresses).await?;

    let mut interval = tokio::time::interval(args.continuous_interval());
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        interval.tick().await;

        info!("Starting continuous handshake cycle");
        let start_time = Instant::now();

        let results = client
            .perform_batch_handshakes(peer_addresses.clone())
            .await;
        let duration = start_time.elapsed();

        let successful_count = results.iter().filter(|(_, result)| result.is_ok()).count();

        info!(
            total_peers = results.len(),
            successful_handshakes = successful_count,
            duration_ms = duration.as_millis(),
            "Continuous handshake cycle completed"
        );
    }
}

/// Run the application in server mode - listen for incoming handshakes
async fn run_server_mode(args: &Args, node_pubkey: Pubkey) -> anyhow::Result<()> {
    info!("Running in server mode - listening for handshakes");

    // Create a gossip node for listening
    let mut gossip_node =
        GossipNode::new(&args.bind_address, node_pubkey, args.network_id.to_string())
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create gossip node: {}", e))?;

    let local_addr = gossip_node
        .local_addr()
        .map_err(|e| anyhow::anyhow!("Failed to get local address: {}", e))?;

    info!(
        local_addr = %local_addr,
        node_pubkey = %node_pubkey,
        "Gossip node listening for handshakes"
    );

    // Run the gossip node event loop
    gossip_node
        .run()
        .await
        .map_err(|e| anyhow::anyhow!("Gossip node error: {}", e))?;

    Ok(())
}

/// Resolve hostname:port strings to SocketAddr
async fn resolve_peer_addresses(peer_addresses: &[String]) -> anyhow::Result<Vec<SocketAddr>> {
    let mut resolved_addrs = Vec::new();

    for addr_str in peer_addresses {
        // Try to resolve hostname to SocketAddr
        match addr_str.to_socket_addrs() {
            Ok(mut addrs) => {
                if let Some(addr) = addrs.next() {
                    resolved_addrs.push(addr);
                } else {
                    return Err(anyhow::anyhow!("Could not resolve address: {}", addr_str));
                }
            }
            Err(e) => {
                return Err(anyhow::anyhow!(
                    "Failed to resolve address '{}': {}",
                    addr_str,
                    e
                ));
            }
        }
    }

    Ok(resolved_addrs)
}
