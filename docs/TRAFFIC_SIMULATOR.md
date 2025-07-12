# Traffic Simulator

The Traffic Simulator is a standalone testing tool that generates realistic message traffic for Azure Service Bus queues. It's designed to help test queue performance, validate processing capabilities, and simulate production-like message loads.

## Overview

The Traffic Simulator operates independently from the main Quetty application and provides:

- **Realistic Traffic Generation**: Produces 60-180 messages per minute with randomized intervals
- **Complete Message Flow**: Sends messages and immediately receives/completes them
- **Real-time Statistics**: Live monitoring of send/receive rates and message counts
- **Flexible Configuration**: Configurable message formats, rates, and display settings
- **Environment Integration**: Uses shared `.env` file for Azure credentials

## Quick Start

### Prerequisites

- Azure Service Bus namespace with a queue
- Encrypted connection string configured in `.env` file (see [Setup](#setup))
- Decryption password for accessing credentials
- Rust development environment

### Setup

1. **Ensure encrypted credentials** are in your `.env` file:
   ```bash
   # These should already be configured if you're using the main Quetty app
   SERVICEBUS__ENCRYPTED_CONNECTION_STRING="encrypted-connection-string-here"
   SERVICEBUS__ENCRYPTION_SALT="base64-encoded-salt-here"
   ```

2. **Know your decryption password** - This is the same password you use for the main Quetty application

### Basic Usage

1. **Run the traffic simulator**:
   ```bash
   make test-server QUEUE=your-queue-name
   ```

2. **Enter password when prompted**:
   ```
   🔒 Found encrypted connection string, prompting for password...
   🔐 Enter decryption password: [hidden input]
   ✅ Successfully decrypted connection string
   ```

3. **Stop the simulator**:
   Press `Ctrl+C` to gracefully shutdown

### Example Output

```
✅ Loading configuration...
✅ Loading connection string...
🚀 Starting Traffic Simulator
📋 Configuration:
   Queue: my-test-queue
   Rate: 60-180 messages/minute
   Message Prefix: TrafficSim
   Format: JSON

🔌 Connecting to Service Bus...
✅ Connected successfully!
🎯 Starting traffic simulation... (Press Ctrl+C to stop)

📊 Traffic Statistics (Running for 1.2 minutes)
   📤 Sent: 112 messages (92.1/min)
   📥 Received: 112 messages (92.1/min)
   🎯 Target Rate: 60-180/min
```

## Configuration

### File-based Configuration

The simulator uses `traffic-simulator/config.toml` for traffic-specific settings:

```toml
[traffic]
min_messages_per_minute = 60
max_messages_per_minute = 180
message_prefix = "TrafficSim"
use_json_format = true

[display]
stats_update_interval_secs = 5
show_message_details = false
```

### Environment Variable Overrides

You can override configuration via environment variables:

```bash
# Override message rate
export TRAFFIC_MIN_RATE=100
export TRAFFIC_MAX_RATE=300

# Override message prefix
export TRAFFIC_MESSAGE_PREFIX="LoadTest"

# Set password via environment (for automation)
export TRAFFIC_PASSWORD="your-decryption-password"

# Override max password attempts
export TRAFFIC_MAX_PASSWORD_ATTEMPTS=5

# Run with overrides
make test-server QUEUE=my-queue
```

### Configuration Options

| Setting | Description | Default | Environment Override |
|---------|-------------|---------|---------------------|
| `min_messages_per_minute` | Minimum message rate | 60 | `TRAFFIC_MIN_RATE` |
| `max_messages_per_minute` | Maximum message rate | 180 | `TRAFFIC_MAX_RATE` |
| `message_prefix` | Prefix for all messages | "TrafficSim" | `TRAFFIC_MESSAGE_PREFIX` |
| `use_json_format` | Send JSON vs text messages | true | - |
| `stats_update_interval_secs` | How often to show stats | 5 | - |
| `max_password_attempts` | Password retry attempts | 3 | `TRAFFIC_MAX_PASSWORD_ATTEMPTS` |
| `encrypted_conn_var` | Encrypted connection env var | `SERVICEBUS__ENCRYPTED_CONNECTION_STRING` | - |
| `salt_var` | Encryption salt env var | `SERVICEBUS__ENCRYPTION_SALT` | - |

## Message Format

### JSON Messages (Default)

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "sequence": 42,
  "timestamp": "2024-01-15T10:30:00Z",
  "prefix": "TrafficSim",
  "content": "Traffic simulation message #42"
}
```

### Text Messages

```
TrafficSim message #42
```

## Use Cases

### Load Testing

Test queue performance under sustained load:

```bash
# High-volume load test
export TRAFFIC_MIN_RATE=500
export TRAFFIC_MAX_RATE=1000
export TRAFFIC_PASSWORD="your-password"  # For automation
make test-server QUEUE=load-test-queue
```

### Development Testing

Generate test data for development:

```bash
# Steady development traffic
export TRAFFIC_MIN_RATE=30
export TRAFFIC_MAX_RATE=60
export TRAFFIC_MESSAGE_PREFIX="Dev"
make test-server QUEUE=dev-queue
```

### Performance Validation

Validate queue processing capabilities:

```bash
# Monitor for processing bottlenecks
make test-server QUEUE=production-like-queue
# Watch for any gaps between sent/received counts
```

## Architecture

The Traffic Simulator is built as a standalone Rust application:

```
traffic-simulator/
├── Cargo.toml          # Independent project dependencies
├── config.toml         # Traffic-specific configuration
└── src/
    ├── main.rs         # Main application with async event loop
    ├── service_bus.rs  # Azure Service Bus client wrapper
    ├── producer.rs     # Message sending functionality
    ├── consumer.rs     # Message receiving functionality
    └── config.rs       # Configuration loading and validation
```

### Key Features

- **Standalone Operation**: Completely independent from main Quetty application
- **Async Architecture**: Built on Tokio for efficient message handling
- **Graceful Shutdown**: Handles Ctrl+C signals cleanly
- **Real-time Monitoring**: Live statistics with configurable update intervals
- **Resource Management**: Proper cleanup of Azure Service Bus connections

## Troubleshooting

### Connection Issues

**Error**: `Failed to create service bus client`
- Verify your encrypted connection string is correct in `.env`
- Check Azure Service Bus namespace is accessible
- Ensure the queue exists
- Verify you're using the correct decryption password

**Error**: `No encrypted connection string found`
- Ensure `SERVICEBUS__ENCRYPTED_CONNECTION_STRING` and `SERVICEBUS__ENCRYPTION_SALT` are set in `.env`
- These should be automatically configured when using the main Quetty application

**Error**: `Failed to decrypt connection string`
- Check that you're using the correct decryption password
- Verify the encryption salt matches the one used to encrypt the connection string

### Permission Issues

**Error**: `InvalidSignature: The token has an invalid signature`
- Verify the SharedAccessKey in your decrypted connection string
- Check that the key hasn't expired
- Ensure you're using the correct namespace
- Re-encrypt your connection string if it has been updated

### Performance Issues

**Slow message rates**
- Check Azure Service Bus throttling limits
- Verify queue isn't hitting capacity limits
- Monitor for network connectivity issues

### Build Issues

**Error**: `traffic-simulator` not found in workspace
- The simulator is intentionally excluded from the main workspace
- Run commands from the project root, not from `traffic-simulator/` directory
- Use `make test-server` rather than direct `cargo run`

**Error**: Compilation issues with encryption dependencies
- Ensure you're using compatible versions of encryption crates
- Try `cargo clean` and rebuild if encountering dependency conflicts

## Integration with Main Application

The Traffic Simulator is designed to work alongside Quetty:

1. **Shared Encrypted Credentials**: Uses the same encrypted `.env` file as the main application
2. **Same Decryption Password**: Uses the same password you set up for Quetty
3. **Independent Operation**: Runs without interfering with main application
4. **Queue Compatibility**: Works with any queue accessible to Quetty
5. **Security Consistency**: Same encryption/security model as main app

## Advanced Usage

### Custom Message Formats

Modify `src/main.rs` to customize message content:

```rust
// In create_message function
let custom_data = json!({
    "custom_field": "your_value",
    "batch_id": batch_identifier,
    "test_scenario": "load_test_v2"
});
```

### Performance Monitoring

Combine with external monitoring tools:

```bash
# Run with custom prefix for tracking
export TRAFFIC_MESSAGE_PREFIX="PerfTest-$(date +%Y%m%d)"
make test-server QUEUE=monitoring-queue
```

### Batch Testing

Test different load patterns:

```bash
# Set password once for all tests
export TRAFFIC_PASSWORD="your-password"

# Low load
export TRAFFIC_MIN_RATE=10 TRAFFIC_MAX_RATE=30
make test-server QUEUE=test-queue

# Medium load
export TRAFFIC_MIN_RATE=100 TRAFFIC_MAX_RATE=200
make test-server QUEUE=test-queue

# High load
export TRAFFIC_MIN_RATE=500 TRAFFIC_MAX_RATE=800
make test-server QUEUE=test-queue
```

### Automated Testing (CI/CD)

For automated environments, set the password via environment variable:

```bash
# In your CI/CD pipeline
export TRAFFIC_PASSWORD="${QUETTY_DECRYPTION_PASSWORD}"
export TRAFFIC_MIN_RATE=100
export TRAFFIC_MAX_RATE=200
make test-server QUEUE=ci-test-queue
```

## Contributing

When contributing to the Traffic Simulator:

1. **Test changes** with various queue configurations
2. **Maintain independence** from main application
3. **Update documentation** for new features
4. **Follow existing patterns** for configuration and error handling

For more information on contributing, see [CONTRIBUTING.md](CONTRIBUTING.md).
