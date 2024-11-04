use broker::net::BrokerClient;
use std::time::Instant;
use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    const MESSAGE_SIZE: usize = 1024;
    const ITERATIONS: usize = 1_000_000;
    
    println!("test_start msg_count:{} msg_bytes:{}", ITERATIONS, MESSAGE_SIZE);
    let mut client = BrokerClient::connect("127.0.0.1:7878").await?;
    
    let mut data = vec![0u8; MESSAGE_SIZE];
    let start = Instant::now();
    
    for i in 0..ITERATIONS {
        // add timestamp, sequence, and calculate cksum
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        data[0..8].copy_from_slice(&now.to_le_bytes());
        
        let sequence = i as u64;
        data[8..16].copy_from_slice(&sequence.to_le_bytes());
        
        let mut hasher = DefaultHasher::new();
        hasher.write_u64(now);
        hasher.write_u64(sequence);
        hasher.write(&data[20..]);
        let checksum = hasher.finish() as u32;
        data[16..20].copy_from_slice(&checksum.to_le_bytes());
        
        client.send(&data).await?;
    }
    
    client.flush().await?;
    
    let elapsed = start.elapsed();
    let throughput = (MESSAGE_SIZE * ITERATIONS) as f64 / elapsed.as_secs_f64();
    
    println!("\nResults (time: {:?}):", elapsed);
    println!("Throughput: {:.2} GB/s", throughput / 1e9);
    println!("Messages/sec: {:.2}", ITERATIONS as f64 / elapsed.as_secs_f64());
    
    Ok(())
}
