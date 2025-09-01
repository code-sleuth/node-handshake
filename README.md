# P2P Solana Handshake

A production-ready Rust implementation of a Solana P2P handshake client and gossip node for participating in Solana network's peer-to-peer communication layer.

## Features

- **Client Mode**: Initiate handshakes with remote Solana gossip nodes
- **Server Mode**: Listen for and respond to incoming handshake requests
- **Multiple Networks**: Support for localnet, testnet, devnet, and mainnet-beta
- **Production Logging**: Structured JSON logging for observability
- **Retry Logic**: Configurable timeouts and exponential backoff
- **Batch Operations**: Handle multiple peer connections simultaneously
- **CLI Interface**: Full-featured command-line interface

## Quick Start

### Prerequisites

- **Rust 1.89+**: Latest stable Rust compiler
- **Docker & Docker Compose**: For containerized deployment
- **Git**: For source code management

### Installation

```bash
git clone git@github.com:code-sleuth/node-handshake.git
cd node-handshake
cargo build --release
```

### Quick Demo

**Try the complete ecosystem in one command:**
```bash
./run_local.sh
```

This starts all services and follows the gossip server logs to show real-time handshake activity.

### Basic Usage

**Option 1: Native Rust (requires separate Solana validator):**

*Terminal 1 - Start Solana test validator:*
```bash
solana-test-validator
```

*Terminal 2 - Run as Server:*
```bash
cargo run -- --mode server --bind 127.0.0.1:8001 --network localnet
```

*Terminal 3 - Run as Client:*
```bash
cargo run -- --mode client --peers 127.0.0.1:8001 --network localnet
```

**Option 2: Docker Compose (fully self-contained):**
```bash
docker-compose up --build
# Everything starts automatically - no additional terminals needed!
```

## Testing Handshakes

### Docker Compose Testing (Recommended)

**Start the complete ecosystem:**
```bash
./run_local.sh
```
This automatically handles all services including the Solana test validator.

### Manual Testing (requires running solana-test-validator)

**Terminal 1 (Solana Test Validator):**
```bash
solana-test-validator
```


If you don't have this installed, follow the instructions here https://solana.com/developers/guides/getstarted/solana-test-validator


**Terminal 2 (Server):**
```bash
cargo run -- --mode server --bind 127.0.0.1:8001 --network localnet --log-level debug
```

**Terminal 3 (Client):**
```bash
cargo run -- --mode client --peers 127.0.0.1:8001 --network localnet --log-level debug
```

### Testing with Pretty Logs (Development)

**Server:**
```bash
cargo run -- --mode server --bind 127.0.0.1:8001 --network localnet --log-format pretty --log-level info
```

**Client:**
```bash
cargo run -- --mode client --peers 127.0.0.1:8001 --network localnet --log-format pretty --log-level info
```

### Testing with JSON Logs (Production-style)

**Server:**
```bash
cargo run -- --mode server --bind 127.0.0.1:8001 --network localnet --log-format json --log-level info
```

**Client:**
```bash
cargo run -- --mode client --peers 127.0.0.1:8001 --network localnet --log-format json --log-level info
```

### Testing Multiple Peers

**Start multiple servers:**
```bash
# Terminal 1
cargo run -- --mode server --bind 127.0.0.1:8001 --network localnet

# Terminal 2
cargo run -- --mode server --bind 127.0.0.1:8002 --network localnet

# Terminal 3
cargo run -- --mode server --bind 127.0.0.1:8003 --network localnet
```

**Connect to all from client:**
```bash
cargo run -- --mode client --peers 127.0.0.1:8001 --peers 127.0.0.1:8002 --peers 127.0.0.1:8003 --network localnet
```

### Continuous Testing

**Client in continuous mode (handshake every 30 seconds):**
```bash
cargo run -- --mode client --peers 127.0.0.1:8001 --network localnet --continuous --interval 30
```

## Expected Output

### Successful Server Logs
```
INFO Starting Solana P2P handshake application
INFO Gossip node listening for handshakes local_addr=127.0.0.1:8001
INFO Received handshake request sender_addr=127.0.0.1:xxxxx peer_pubkey=ABC123...
INFO Handshake response sent peer_addr=127.0.0.1:xxxxx success=true
```

### Successful Client Logs
```
INFO Starting Solana P2P handshake application
INFO Running in client mode peer_count=1
INFO Starting handshake peer_addr=127.0.0.1:8001
INFO Handshake completed successfully peer_addr=127.0.0.1:8001 peer_pubkey=DEF456...
INFO Batch handshakes completed total_peers=1 successful_handshakes=1
```

## Testing Different Scenarios

### Testing Different Networks

**Test network mismatch (should fail):**
```bash
# Server on testnet
cargo run -- --mode server --bind 127.0.0.1:8001 --network testnet

# Client on localnet (will fail with network mismatch error)
cargo run -- --mode client --peers 127.0.0.1:8001 --network localnet
```

### Testing Error Scenarios

**Test timeout:**
```bash
# Client with very short timeout
cargo run -- --mode client --peers 127.0.0.1:8001 --network localnet --timeout 1
```

**Test connection to non-existent peer:**
```bash
cargo run -- --mode client --peers 127.0.0.1:9999 --network localnet --log-level debug
```

### Debugging with Trace Logs

For maximum debugging output:
```bash
cargo run -- --mode server --bind 127.0.0.1:8001 --network localnet --log-level trace
cargo run -- --mode client --peers 127.0.0.1:8001 --network localnet --log-level trace
```

## Command Line Options

### Core Options
- `--bind, -b`: Local address to bind (default: "0.0.0.0:0")
- `--network, -n`: Network ID (localnet, testnet, devnet, mainnet-beta)
- `--peers, -p`: Remote peer addresses (can specify multiple)
- `--mode, -m`: Application mode (client or server)

### Configuration
- `--timeout, -t`: Operation timeout in seconds (default: 30)
- `--max-retries, -r`: Maximum retry attempts incase of failure (default: 3)
- `--continuous, -c`: Run in continuous mode
- `--interval, -i`: Interval between handshakes in continuous mode (default: 60s)

### Logging
- `--log-level, -l`: Log level (trace, debug, info, warn, error)
- `--log-format`: Log format (pretty or json)

## Docker Compose Setup

The project includes a complete Docker Compose setup that demonstrates a multi-node Solana P2P handshake ecosystem with:

- **Solana Test Validator**: Local Solana network for testing
- **Gossip Server**: Central server listening for handshake requests
- **Multiple Clients**: 4 clients with different handshake intervals (30s, 45s, 60s, 120s)
- **Structured Logging**: Comprehensive logging with JSON and pretty formats
- **Service Discovery**: Container networking for peer communication

### Quick Start with Docker Compose

**Start the entire ecosystem (no additional setup required):**

```bash
./run_local.sh
```

This will automatically start:
- 1x **Solana test validator** (ports 8899, 8900)
- 1x **Gossip server** (UDP port 8000)
- 4x **Client containers** with different handshake intervals (30s, 45s, 60s, 120s)

> **Note**: The Docker Compose setup includes its own containerized Solana test validator, so you don't need to run `solana-test-validator` in a separate terminal.

### Container Architecture

```
┌─────────────────┐    ┌─────────────────┐
│ solana-test-    │    │   gossip-server │◄──┐
│ validator       │    │   (UDP :8000)   │   │
│ (RPC: 8899-8900)│    └─────────────────┘   │
└─────────────────┘                          │
                                             │ Handshake
┌─────────────────┐    ┌─────────────────┐   │ Requests
│    client-1     │    │    client-2     │───┤
│  (30s interval) │    │  (45s interval) │   │
└─────────────────┘    └─────────────────┘   │
                                             │
┌─────────────────┐    ┌─────────────────┐   │
│    client-3     │    │    client-4     │───┘
│  (60s interval) │    │ (120s interval) │
└─────────────────┘    └─────────────────┘
```

### Container Management

**Start all services:**
```bash
./run_local.sh
```

**View running containers:**
```bash
docker-compose ps
```

### Monitoring Container Logs

**View all logs:**
```bash
docker-compose logs
```

**Follow gossip server logs:**
```bash
docker-compose logs -f gossip-server
```

**View specific client logs:**
```bash
docker-compose logs -f client-1
docker-compose logs -f client-2
```

### Expected Docker Output

**Gossip Server Activity:**
```
INFO Initializing gossip node bind_addr=0.0.0.0:8000 node_pubkey=1111111QLbz7JHiBTspS962RLKV8GndWFwiEaqKM
INFO Gossip node listening for handshakes local_addr=0.0.0.0:8000
INFO Received handshake request sender_addr=172.19.0.4:51807 peer_pubkey=1111111QLbz7JHiBTspS962RLKV8GndWFwiEaqKM
INFO Peer registry updated total_peers=4
```

**Client Activity:**
```
INFO Running in continuous client mode interval_secs=30
INFO Starting handshake peer_addr=172.19.0.3:8000
INFO Handshake completed successfully peer_addr=172.19.0.3:8000 attempt=1
INFO Batch handshakes completed total_peers=1 successful_handshakes=1
```

## Development

### Building
```bash
cargo build              # Debug build
cargo build --release    # Release build
```

### Testing
```bash
cargo test               # Run tests
cargo test --verbose     # Verbose test output
```

### Code Quality
```bash
cargo check              # Quick compile check
cargo clippy             # Linting
cargo fmt               # Code formatting
```

## Architecture

The application is structured as both a library and binary:

- **Library (`src/lib.rs`)**: Reusable components for integration
- **Binary (`src/main.rs`)**: CLI application
- **Modular Design**: Domain-driven module organization
- **Production Ready**: Enterprise-grade error handling and logging

### Key Components

- **GossipNode**: UDP server for handling incoming handshakes
- **HandshakeClient**: Client for initiating peer connections
- **Domain Models**: Type-safe data structures and validation
- **Telemetry**: Structured logging and observability

## Networks Supported

- **Localnet**: Local development network
- **Testnet**: Solana testnet environment
- **Devnet**: Solana development network
- **Mainnet-beta**: Solana production network

## Recent Improvements

### Docker Compose Enhancements

- **Fixed Binary Path Issues**: Resolved Docker container binary naming inconsistencies
- **Multi-Client Architecture**: Added 4 client containers with different handshake intervals
- **Service Dependencies**: Proper container startup ordering and health checks
- **Network Isolation**: Dedicated Docker network for service communication
- **Comprehensive Logging**: Structured logging with log rotation and size limits

### Container Features

- **Cargo Chef Integration**: Fast Docker builds with dependency caching
- **Multi-Stage Builds**: Optimized container sizes using Alpine Linux and Debian slim
- **Auto-restart Policies**: Containers automatically restart on failure
- **Port Management**: Proper UDP port exposure for gossip protocol
- **Resource Limits**: JSON file logging with size and rotation limits

### Monitoring & Observability

- **Structured Telemetry**: Enhanced logging with correlation IDs and context
- **Real-time Monitoring**: Live log streaming and container status monitoring
- **Debugging Support**: Multiple log levels and formats for development and production
- **Performance Metrics**: Handshake timing and success rate tracking

## Performance & Scaling

### Container Performance

The Docker setup is optimized for performance:

- **Resource Efficiency**: Containers use minimal resources (< 10MB RAM per client)
- **Fast Startup**: Cargo chef caching reduces build times from 5+ minutes to < 30 seconds on rebuilds
- **Concurrent Handshakes**: Server handles multiple concurrent client connections efficiently
- **UDP Protocol**: Low-latency UDP communication for optimal handshake performance

### Scaling Considerations

**Horizontal Scaling:**
```bash
# Scale client containers
docker-compose up --scale client-1=3 --scale client-2=2

# Add more gossip servers
docker-compose up gossip-server-2 gossip-server-3
```

**Load Testing:**
- Server tested with 4+ concurrent clients
- Handles rapid handshake cycles (30-second intervals)
- Maintains peer registry with connection status tracking
- Graceful handling of network timeouts and retries
