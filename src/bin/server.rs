use broker::net::BrokerServer;
use tokio::signal;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = BrokerServer::new(7878);
    println!("broker server started");
    tokio::select! {
        res = server.run() => {
            if let Err(e) = res {
                eprintln!("Server error: {}", e);
            }
        }
        _ = signal::ctrl_c() => {
            println!("\nShutting down...");
        }
    }
    Ok(())
}

