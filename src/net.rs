use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::sync::Arc;
use thiserror::Error;

use crate::RingBuffer;

#[derive(Error, Debug)]
pub enum NetworkError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Broker error: {0}")]
    Broker(#[from] crate::BrokerError),
    
    #[error("Connection closed")]
    ConnectionClosed,
}

/// used for network messages
///
#[repr(C, packed)]
struct MessageHeader {
    size: u32,
    flags: u32,
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
        println!("Listening on {}", addr);

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
    let mut msg_buf = Vec::with_capacity(16 * 1024); 
    
    loop {
        if socket.read_exact(&mut header_buf).await? == 0 {
            return Err(NetworkError::ConnectionClosed);
        }
        
        let header = unsafe { 
            std::ptr::read_unaligned(header_buf.as_ptr() as *const MessageHeader)
        };
        
        // may need to  resize buffer
        if msg_buf.capacity() < header.size as usize {
            msg_buf.reserve(header.size as usize);
        }
        
        // read message body
        unsafe { msg_buf.set_len(header.size as usize) };
        socket.read_exact(&mut msg_buf).await?;
        
        while ring.try_write(&msg_buf).is_err() {
            tokio::task::yield_now().await;
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
        
        Ok(Self { stream })
    }
    pub async fn send(&mut self, data: &[u8]) -> Result<(), NetworkError> {
        let header = MessageHeader {
            size: data.len() as u32,
            flags: 0,
        };
        
        let header_bytes = unsafe {
            std::slice::from_raw_parts(
                &header as *const _ as *const u8,
                std::mem::size_of::<MessageHeader>()
            )
        };
        self.stream.write_all(header_bytes).await?;
        self.stream.write_all(data).await?;
        
        Ok(())
    }
}
