use broker::net::BrokerClient;
use std::time::Instant;
use std::io::Write;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    const MESSAGE_SIZE: usize = 1024;
    const ITERATIONS: usize = 10_000;  // reduced for testing
    
    println!("Connecting to broker...");
    let mut client = BrokerClient::connect("127.0.0.1:7878").await?;
    println!("connected");
    
    let data = vec![1u8; MESSAGE_SIZE];
    let start = Instant::now();
    
    for i in 0..ITERATIONS {
        if i % 100 == 0 {  // More frequent updates
            print!("\rProgress: {:.1}%", (i as f64 / ITERATIONS as f64) * 100.0);
            std::io::stdout().flush()?;
        }

        match client.send(&data).await {
            Ok(_) => {
                if i % 1000 == 0 {
                    println!("\nSent {} messages", i);
                }
            }
            Err(e) => {
                println!("\nError at iteration {}: {:?}", i, e);
                break;
            }
        }
    }
    
    let elapsed = start.elapsed();
    let throughput = (MESSAGE_SIZE * ITERATIONS) as f64 / elapsed.as_secs_f64();
    
    println!("\nResults:");
    println!("Time: {:?}", elapsed);
    println!("Throughput: {:.2} GB/s", throughput / 1e9);
    println!("Messages/sec: {:.2}", ITERATIONS as f64 / elapsed.as_secs_f64());
    
    Ok(())
}
