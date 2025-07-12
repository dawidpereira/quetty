mod config;
mod consumer;
mod password;
mod producer;
mod service_bus;

use chrono::Utc;
use config::Config;
use producer::Producer;
use serde_json::json;
use service_bus::ServiceBusManager;
use std::{env, time::Duration};
use tokio::signal;

#[derive(Debug)]
struct TrafficStats {
    sent_count: u32,
    received_count: u32,
    start_time: chrono::DateTime<Utc>,
}

impl TrafficStats {
    fn new() -> Self {
        Self {
            sent_count: 0,
            received_count: 0,
            start_time: Utc::now(),
        }
    }

    fn display(&self, config: &config::TrafficConfig) {
        let elapsed = Utc::now().signed_duration_since(self.start_time);
        let elapsed_mins =
            elapsed.num_minutes() as f64 + (elapsed.num_seconds() % 60) as f64 / 60.0;

        let send_rate = if elapsed_mins > 0.0 {
            self.sent_count as f64 / elapsed_mins
        } else {
            0.0
        };
        let recv_rate = if elapsed_mins > 0.0 {
            self.received_count as f64 / elapsed_mins
        } else {
            0.0
        };

        println!(
            "📊 Traffic Statistics (Running for {:.1} minutes)",
            elapsed_mins
        );
        println!(
            "   📤 Sent: {} messages ({:.1}/min)",
            self.sent_count, send_rate
        );
        println!(
            "   📥 Received: {} messages ({:.1}/min)",
            self.received_count, recv_rate
        );
        println!(
            "   🎯 Target Rate: {}-{}/min",
            config.min_messages_per_minute, config.max_messages_per_minute
        );
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <queue-name>", args[0]);
        std::process::exit(1);
    }

    let queue_name = &args[1];

    println!("✅ Loading configuration...");
    let config = match Config::load() {
        Ok(config) => config,
        Err(e) => {
            eprintln!("❌ Failed to load config.toml: {}", e);
            std::process::exit(1);
        }
    };

    println!("✅ Loading connection string...");
    let connection_string = match config.load_connection_string() {
        Ok(conn_str) => conn_str,
        Err(e) => {
            eprintln!("❌ {}", e);
            std::process::exit(1);
        }
    };

    println!("🚀 Starting Traffic Simulator");
    println!("📋 Configuration:");
    println!("   Queue: {}", queue_name);
    println!(
        "   Rate: {}-{} messages/minute",
        config.traffic.min_messages_per_minute, config.traffic.max_messages_per_minute
    );
    println!("   Message Prefix: {}", config.traffic.message_prefix);
    println!(
        "   Format: {}",
        if config.traffic.use_json_format {
            "JSON"
        } else {
            "Text"
        }
    );
    println!();

    println!("🔌 Connecting to Service Bus...");
    let mut service_bus = match ServiceBusManager::new(&connection_string).await {
        Ok(sb) => sb,
        Err(e) => {
            eprintln!("❌ Failed to create service bus client: {}", e);
            std::process::exit(1);
        }
    };

    let mut producer = match service_bus.create_producer(queue_name).await {
        Ok(p) => p,
        Err(e) => {
            eprintln!("❌ Failed to create producer: {}", e);
            std::process::exit(1);
        }
    };

    let mut consumer = match service_bus.create_consumer(queue_name).await {
        Ok(c) => c,
        Err(e) => {
            eprintln!("❌ Failed to create consumer: {}", e);
            std::process::exit(1);
        }
    };

    println!("✅ Connected successfully!");
    println!("🎯 Starting traffic simulation... (Press Ctrl+C to stop)");
    println!();

    let mut stats = TrafficStats::new();
    let mut last_stats_display = std::time::Instant::now();

    // Main traffic loop
    loop {
        tokio::select! {
            _ = signal::ctrl_c() => {
                println!("\n🛑 Shutting down gracefully...");
                break;
            }
            _ = tokio::time::sleep(Duration::from_millis(calculate_delay_ms(&config.traffic))) => {
                // Send a message
                let message = create_message(&config.traffic, stats.sent_count + 1);
                if let Err(e) = producer.send_message(message).await {
                    eprintln!("❌ Failed to send message: {}", e);
                } else {
                    stats.sent_count += 1;
                }

                // Try to receive and complete a message
                match consumer.receive_messages_with_timeout(1, Duration::from_secs(1)).await {
                    Ok(messages) => {
                        for message in messages {
                            if let Err(e) = consumer.complete_message(&message).await {
                                eprintln!("❌ Failed to complete message: {}", e);
                            } else {
                                stats.received_count += 1;
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("❌ Failed to receive messages: {}", e);
                    }
                }

                // Display stats periodically
                if last_stats_display.elapsed() >= Duration::from_secs(config.display.stats_update_interval_secs) {
                    stats.display(&config.traffic);
                    println!();
                    last_stats_display = std::time::Instant::now();
                }
            }
        }
    }

    // Cleanup
    if let Err(e) = producer.dispose().await {
        eprintln!("Warning: Failed to dispose producer: {}", e);
    }
    if let Err(e) = consumer.dispose().await {
        eprintln!("Warning: Failed to dispose consumer: {}", e);
    }

    println!("📊 Final Statistics:");
    stats.display(&config.traffic);
    println!("✅ Traffic simulator stopped.");

    Ok(())
}

fn calculate_delay_ms(config: &config::TrafficConfig) -> u64 {
    let messages_per_minute =
        fastrand::u32(config.min_messages_per_minute..=config.max_messages_per_minute);
    // Convert to delay between messages in milliseconds
    (60_000 / messages_per_minute as u64).max(100) // Min 100ms delay
}

fn create_message(
    config: &config::TrafficConfig,
    sequence: u32,
) -> azservicebus::ServiceBusMessage {
    if config.use_json_format {
        let data = json!({
            "id": uuid::Uuid::new_v4().to_string(),
            "sequence": sequence,
            "timestamp": Utc::now().to_rfc3339(),
            "prefix": config.message_prefix,
            "content": format!("Traffic simulation message #{}", sequence)
        });
        Producer::create_json_message(&data).unwrap_or_else(|_| {
            Producer::create_text_message(&format!(
                "{} message #{}",
                config.message_prefix, sequence
            ))
        })
    } else {
        Producer::create_text_message(&format!("{} message #{}", config.message_prefix, sequence))
    }
}
