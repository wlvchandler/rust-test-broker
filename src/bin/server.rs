use broker::BrokerServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut server = BrokerServer::new(7878);
    println!("starting server");
    server.run().await?;
    Ok(())
}
