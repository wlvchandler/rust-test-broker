use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::time::{timeout, Duration};
use std::sync::Arc;
use thiserror::Error;

use crate::RingBuffer;

const TIMEOUT_SECS: u64 = 1;

#[derive(Error, Debug)]
pub enum NetworkError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Broker error: {0}")]
    Broker(#[from] crate::BrokerError),
    #[error("Timeout")]
    Timeout,
}

#[repr(C, packed)]
struct MessageHeader {
    size: u32,
}

impl MessageHeader {
    fn size(&self) -> u32 {
        unsafe { 
            let ptr = (self as *const MessageHeader).cast::<u8>();
            std::ptr::read_unaligned(ptr.cast::<u32>())
        }
    }
}

pub struct BrokerServer {
    ring: Arc<RingBuffer>,
    port: u16,
}

impl BrokerServer {
    pub fn new(port: u16) -> Self {
        Self {
            ring: Arc::new(RingBuffer::new().expect("Failed to create ring buffer")),
            port,
        }
    }

    pub async fn run(&self) -> Result<(), NetworkError> {
        let addr = format!("0.0.0.0:{}", self.port);
        let listener = TcpListener::bind(&addr).await?;
        println!("Server listening on {}", addr);

        loop {
            let (socket, addr) = listener.accept().await?;
            println!("New connection from {}", addr);
            
            let ring = self.ring.clone();
            tokio::spawn(async move {
                if let Err(e) = handle_connection(socket, ring).await {
                    eprintln!("Connection error: {}", e);
                }
            });
        }
    }
}

async fn handle_connection(mut socket: TcpStream, ring: Arc<RingBuffer>) -> Result<(), NetworkError> {
    socket.set_nodelay(true)?;
    let mut header_buf = [0u8; std::mem::size_of::<MessageHeader>()];
    let mut msg_buf = vec![0u8; 1024];  // Start with smaller buffer
    let mut count = 0;

    loop {
        match timeout(Duration::from_secs(TIMEOUT_SECS), socket.read_exact(&mut header_buf)).await {
            Ok(Ok(_)) => {
                let header = unsafe {
                    std::ptr::read_unaligned(header_buf.as_ptr() as *const MessageHeader)
                };
                let size = header.size() as usize;
                println!("Server received header, size: {}", size);

                if msg_buf.len() < size {
                    msg_buf.resize(size, 0);
                }

                match timeout(Duration::from_secs(TIMEOUT_SECS), socket.read_exact(&mut msg_buf[..size])).await {
                    Ok(Ok(_)) => {
                        while ring.try_write(&msg_buf[..size]).is_err() {
                            tokio::task::yield_now().await;
                        }
                        count += 1;
                        if count % 10000 == 0 {
                            println!("Server processed {} messages", count);
                        }
                    }
                    Ok(Err(e)) => {
                        println!("Server error reading message: {}", e);
                        return Err(e.into());
                    }
                    Err(_) => {
                        println!("Server timeout reading message");
                        return Err(NetworkError::Timeout);
                    }
                }
            }
            Ok(Err(e)) => {
                println!("Server error reading header: {}", e);
                return Err(e.into());
            }
            Err(_) => {
                println!("Server timeout reading header");
                return Err(NetworkError::Timeout);
            }
        }
    }
}

pub struct BrokerClient {
    stream: TcpStream,
}

impl BrokerClient {
    pub async fn connect(addr: &str) -> Result<Self, NetworkError> {
        let stream = TcpStream::connect(addr).await?;
        stream.set_nodelay(true)?;
        println!("Connected to {}", addr);
        Ok(Self { stream })
    }

    pub async fn send(&mut self, data: &[u8]) -> Result<(), NetworkError> {
        let header = MessageHeader {
            size: data.len() as u32,
        };

        let header_bytes = unsafe {
            std::slice::from_raw_parts(
                &header as *const _ as *const u8,
                std::mem::size_of::<MessageHeader>()
            )
        };

        match timeout(Duration::from_secs(TIMEOUT_SECS), self.stream.write_all(header_bytes)).await {
            Ok(Ok(_)) => {},
            Ok(Err(e)) => {
                println!("Client error writing header: {}", e);
                return Err(e.into());
            }
            Err(_) => {
                println!("Client timeout writing header");
                return Err(NetworkError::Timeout);
            }
        }

        match timeout(Duration::from_secs(TIMEOUT_SECS), self.stream.write_all(data)).await {
            Ok(Ok(_)) => {},
            Ok(Err(e)) => {
                println!("Client error writing data: {}", e);
                return Err(e.into());
            }
            Err(_) => {
                println!("Client timeout writing data");
                return Err(NetworkError::Timeout);
            }
        }

        Ok(())
    }
}
