use broker::BrokerClient;
use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::pin;
use tokio::signal;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    const MESSAGE_SIZE: usize = 1024;

    println!("Starting client. Press Ctrl+C to stop.");
    println!("test_start msg_bytes:{}", MESSAGE_SIZE);

    // Connect to the broker server
    let mut client = BrokerClient::connect("127.0.0.1:7878").await?;

    // Initialize the data buffer
    let mut data = vec![0u8; MESSAGE_SIZE];
    let start = Instant::now();

    // Atomic counters to track messages and bytes sent
    let messages_sent = Arc::new(AtomicU64::new(0));
    let bytes_sent = Arc::new(AtomicU64::new(0));

    // Clone counters for use inside the loop
    let messages_sent_clone = messages_sent.clone();
    let bytes_sent_clone = bytes_sent.clone();

    // Create the shutdown signal and pin it
    let shutdown_signal = signal::ctrl_c();
    pin!(shutdown_signal);

    let mut i: u64 = 0;

    loop {
        tokio::select! {
            // Listen for the shutdown signal (Ctrl+C)
            _ = &mut shutdown_signal => {
                println!("\nShutdown signal received. Stopping client...");
                break;
            }

            // Prepare and send the message
            result = async {
                // Add timestamp
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos() as u64;
                data[0..8].copy_from_slice(&now.to_le_bytes());

                // Add sequence number
                let sequence = i;
                data[8..16].copy_from_slice(&sequence.to_le_bytes());

                // Calculate checksum
                let mut hasher = DefaultHasher::new();
                hasher.write_u64(now);
                hasher.write_u64(sequence);
                hasher.write(&data[20..]);
                let checksum = hasher.finish() as u32;
                data[16..20].copy_from_slice(&checksum.to_le_bytes());

                // Send the message
                client.send(&data).await
            } => {
                match result {
                    Ok(_) => {
                        // Increment counters
                        messages_sent_clone.fetch_add(1, Ordering::Relaxed);
                        bytes_sent_clone.fetch_add(MESSAGE_SIZE as u64, Ordering::Relaxed);
                        i += 1;
                    }
                    Err(e) => {
                        eprintln!("Error sending message: {:?}", e);
                        break;
                    }
                }
            }
        }
    }

    // Flush any remaining messages
    if let Err(e) = client.flush().await {
        eprintln!("Error flushing client: {:?}", e);
    }

    // Calculate elapsed time
    let elapsed = start.elapsed();

    // Retrieve counters
    let total_messages = messages_sent.load(Ordering::Relaxed);
    let total_bytes = bytes_sent.load(Ordering::Relaxed);

    // Calculate throughput and messages per second
    let throughput = total_bytes as f64 / elapsed.as_secs_f64();
    let messages_per_sec = total_messages as f64 / elapsed.as_secs_f64();

    // Display statistics
    println!("\nResults (time: {:?}):", elapsed);
    println!("Throughput: {:.2} GB/s", throughput / 1e9);
    println!("Messages/sec: {:.2}", messages_per_sec);
    println!("Total messages sent: {}", total_messages);
    println!("Total bytes sent: {}", total_bytes);

    Ok(())
}
