use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufWriter};
use std::sync::Arc;
use std::hint::black_box;
use thiserror::Error;
use std::hash::Hasher;

use crate::{RingBuffer, BATCH_SIZE, BUFFER_CHUNK};

#[derive(Error, Debug)]
pub enum NetworkError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Broker error: {0}")]
    Broker(#[from] crate::BrokerError),
}

#[derive(Debug)]
struct ProcessedMessage {
    timestamp: u64,
    sequence: u64,
    checksum: u32,
    payload: Vec<u8>,
}

impl ProcessedMessage {
    fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < 20 { return None; }

        let timestamp = u64::from_le_bytes(data[0..8].try_into().ok()?);
        let sequence = u64::from_le_bytes(data[8..16].try_into().ok()?);
        let checksum = u32::from_le_bytes(data[16..20].try_into().ok()?);

        // validate checksum
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        hasher.write_u64(timestamp);
        hasher.write_u64(sequence);
        hasher.write(&data[20..]);
        let computed_hash = hasher.finish() as u32;

        if computed_hash != checksum {
            return None;
        }

        Some(ProcessedMessage {
            timestamp,
            sequence,
            checksum,
            payload: data[20..].to_vec(),
        })
    }

    // some processing to actually use the fields
    fn process(&self) -> bool {
        // verify message is recent (~1s)
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;

        let age_nanos = now - self.timestamp;
        if age_nanos > 1_000_000_000 {
            return false;
        }

        // doing something with payload, to ensure it gets processed (just verifying tcp is
        // actually being used)
        let sum: u32 = self.payload.iter()
            .enumerate()
            .map(|(i, &b)| b as u32 * i as u32)
            .sum();
        black_box(sum);

        true
    }
}

#[repr(C, packed)]
struct MessageHeader {
    size: u32,
    batch_size: u32,
}

impl MessageHeader {
    #[inline(always)]
    fn size(&self) -> u32 {
        unsafe { 
            let ptr = (self as *const MessageHeader).cast::<u8>();
            std::ptr::read_unaligned(ptr.cast::<u32>())
        }
    }

    #[inline(always)]
    fn batch_size(&self) -> u32 {
        unsafe { 
            let ptr = (self as *const MessageHeader).cast::<u8>();
            std::ptr::read_unaligned(ptr.add(4).cast::<u32>())
        }
    }
}

pub struct BrokerServer {
    ring: Arc<RingBuffer>,
    port: u16,
}

impl BrokerServer {
    pub fn new(port: u16) -> Self {
        eprintln!("DEBUG: Creating new BrokerServer on port {}", port);
        let ring = Arc::new(RingBuffer::new().expect("Failed to create ring buffer"));

        let consumer_ring = ring.clone();
        tokio::spawn(async move {
            eprintln!("DEBUG: Starting consumer task");
            let mut msg_bufs = [
                vec![0u8; BUFFER_CHUNK * BATCH_SIZE],
                vec![0u8; BUFFER_CHUNK * BATCH_SIZE],
            ];
            let mut current_buf = 0;
            let mut messages_consumed = 0;
            let mut messages_processed = 0;
            let mut processing_errors = 0;

            loop {
                let mut batch_size = 0;
                let buf = &mut msg_bufs[current_buf];

                for _ in 0..BATCH_SIZE {
                    match consumer_ring.try_read(&mut buf[batch_size..]) {
                        Ok(size) => {
                            match ProcessedMessage::from_bytes(&buf[batch_size..batch_size+size]) {
                                Some(msg) => {
                                    if msg.process() {
                                        messages_processed += 1;
                                    } else {
                                        processing_errors += 1;
                                    }
                                },
                                None => {
                                    processing_errors += 1;
                                }
                            }

                            batch_size += size;
                            messages_consumed += 1;
                        },
                        Err(_) => {
                            if batch_size > 0 { break }
                            tokio::task::yield_now().await;
                            continue;
                        },
                    }
                }

                if messages_consumed % 1_000_000 == 0 && messages_consumed > 0 {
                    eprintln!(
                        "Stats: consumed={}, processed={}, errors={}",
                        messages_consumed, messages_processed, processing_errors
                    );
                }

                current_buf = 1 - current_buf;
            }
        });

        Self { ring, port }
    }

    pub async fn run(&self) -> Result<(), NetworkError> {
        let addr = format!("0.0.0.0:{}", self.port);
        let listener = TcpListener::bind(&addr).await?;
        println!("listening on {}", addr);

        loop {
            let (socket, addr) = listener.accept().await?;
            println!("new connection - from {}", addr);

            let ring = self.ring.clone();
            tokio::spawn(async move {
                if let Err(e) = handle_connection(socket, ring).await {
                    eprintln!("Connection error: {:?}", e);
                }
            });
        }
    }
}

async fn handle_connection(mut socket: TcpStream, ring: Arc<RingBuffer>) -> Result<(), NetworkError> {
    socket.set_nodelay(true)?;

    let mut header_buf = [0u8; std::mem::size_of::<MessageHeader>()];
    let mut msg_buf = vec![0u8; BUFFER_CHUNK * BATCH_SIZE];
    let mut messages_handled = 0;

    loop {
        socket.read_exact(&mut header_buf).await?;
        let header = unsafe {
            std::ptr::read_unaligned(header_buf.as_ptr() as *const MessageHeader)
        };

        let total_size = header.size() as usize * header.batch_size() as usize;
        if msg_buf.len() < total_size {
            msg_buf.resize(total_size, 0);
        }

        socket.read_exact(&mut msg_buf[..total_size]).await?;

        let mut offset = 0;
        for _ in 0..header.batch_size() {
            let msg_size = header.size() as usize;
            while ring.try_write(&msg_buf[offset..offset + msg_size]).is_err() {
                tokio::task::yield_now().await;
            }
            offset += msg_size;
            messages_handled += 1;
        }

        if messages_handled % 1_000_000 == 0 {
            eprintln!("INFO: Server processed {} million messages",
                messages_handled / 1_000_000);
        }
    }
}

pub struct BrokerClient {
    writer: BufWriter<TcpStream>,
    batch: Vec<u8>,
    batch_count: u32,
    total_sent: u64,
}

impl BrokerClient {
    pub async fn connect(addr: &str) -> Result<Self, NetworkError> {
        eprintln!("DEBUG: Connecting to {}", addr);
        let stream = TcpStream::connect(addr).await?;
        stream.set_nodelay(true)?;
        eprintln!("SUCCESS: Connected to {}", addr);
        
        Ok(Self { 
            writer: BufWriter::with_capacity(BUFFER_CHUNK * 4, stream),
            batch: Vec::with_capacity(BUFFER_CHUNK * BATCH_SIZE),
            batch_count: 0,
            total_sent: 0,
        })
    }

    #[inline]
    pub async fn send(&mut self, data: &[u8]) -> Result<(), NetworkError> {
        self.batch.extend_from_slice(data);
        self.batch_count += 1;
        self.total_sent += 1;

        if self.batch_count >= BATCH_SIZE as u32 {
            let header = MessageHeader {
                size: data.len() as u32,
                batch_size: self.batch_count,
            };

            let header_bytes = unsafe {
                std::slice::from_raw_parts(
                    &header as *const _ as *const u8,
                    std::mem::size_of::<MessageHeader>()
                )
            };

            self.writer.write_all(header_bytes).await?;
            self.writer.write_all(&self.batch).await?;
            self.writer.flush().await?;

            self.batch.clear();
            self.batch_count = 0;

            if self.total_sent % 1_000_000 == 0 {
                eprintln!("DEBUG: Client sent {} million messages", 
                    self.total_sent / 1_000_000);
            }
        }

        Ok(())
    }

    pub async fn flush(&mut self) -> Result<(), NetworkError> {
        if self.batch_count > 0 {
            let msg_size = self.batch.len() / self.batch_count as usize;
            let header = MessageHeader {
                size: msg_size as u32,
                batch_size: self.batch_count,
            };

            let header_bytes = unsafe {
                std::slice::from_raw_parts(
                    &header as *const _ as *const u8,
                    std::mem::size_of::<MessageHeader>()
                )
            };

            self.writer.write_all(header_bytes).await?;
            self.writer.write_all(&self.batch).await?;
            self.writer.flush().await?;

            self.batch.clear();
            self.batch_count = 0;
        }
        Ok(())
    }
}
