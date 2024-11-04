use broker::net::BrokerServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = BrokerServer::new(7878);
    println!("starting server");
    server.run().await?;
    Ok(())
}
