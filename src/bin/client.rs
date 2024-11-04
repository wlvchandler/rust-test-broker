use broker::net::BrokerClient;
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = BrokerClient::connect("127.0.0.1:7878").await?;
    
    const MESSAGE_SIZE: usize = 1024;
    const ITERATIONS: usize = 1_000_000;
    
    println!("connected");
    println!("sending: msg_n:{} bytes:{}...", ITERATIONS, MESSAGE_SIZE);
    
    let data = vec![1u8; MESSAGE_SIZE];
    let start = Instant::now();
    
    for i in 0..ITERATIONS {
        if i % (ITERATIONS / 10) == 0 {
            print!("{}%...", (i * 100) / ITERATIONS);
            let _ = std::io::Write::flush(&mut std::io::stdout());
        }
        
        client.send(&data).await?;
    }
    
    let elapsed = start.elapsed();
    let throughput = (MESSAGE_SIZE * ITERATIONS) as f64 / elapsed.as_secs_f64();
    println!("\nResults:");
    println!("Time: {:?}", elapsed);
    println!("Throughput: {:.2} GB/s", throughput / 1e9);
    println!("Messages/sec: {:.2}", ITERATIONS as f64 / elapsed.as_secs_f64());
    
    Ok(())
}
