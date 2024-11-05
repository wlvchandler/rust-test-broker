use crate::error::{BrokerError, NetworkError};
use crate::net::message::{MessageHeader, ProcessedMessage};
use crate::RingBuffer;
use crate::{BATCH_SIZE, BUFFER_CHUNK};
use std::hint::black_box;
use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::watch;

pub struct BrokerServer {
    ring: Arc<RingBuffer>,
    port: u16,
    consumer_shutdown: watch::Sender<bool>,
}

impl BrokerServer {
    pub fn new(port: u16) -> Self {
        eprintln!("DEBUG: Creating new BrokerServer on port {}", port);
        let ring = Arc::new(RingBuffer::new().expect("Failed to create ring buffer"));
        let (shutdown_tx, _) = watch::channel(false);

        Self {
            ring,
            port,
            consumer_shutdown: shutdown_tx,
        }
    }

    pub async fn run(&mut self) -> Result<(), NetworkError> {
        let addr = format!("0.0.0.0:{}", self.port);
        let listener = TcpListener::bind(&addr).await?;
        println!("Server listening on {}", addr);

        loop {
            let (socket, addr) = listener.accept().await?;
            println!("New connection from {}", addr);

            // Stop previous consumer
            let _ = self.consumer_shutdown.send(true);

            // Create new shutdown channel
            let (shutdown_tx, shutdown_rx) = watch::channel(false);
            self.consumer_shutdown = shutdown_tx;

            // Start new consumer
            let ring = self.ring.clone();
            tokio::spawn(async move {
                let result = handle_connection(socket, ring, shutdown_rx).await;
                if let Err(e) = result {
                    eprintln!("Connection error: {:?}", e);
                }
            });
        }
    }
}

async fn handle_connection(
    mut socket: TcpStream,
    ring: Arc<RingBuffer>,
    mut shutdown: watch::Receiver<bool>,
) -> Result<(), NetworkError> {
    socket.set_nodelay(true)?;

    // Spawn consumer task
    let consumer_ring = ring.clone();
    let consumer_shutdown = shutdown.clone();

    let consumer = tokio::spawn(async move {
        println!("DEBUG: Starting consumer task");
        let mut msg_bufs = [
            vec![0u8; BUFFER_CHUNK * BATCH_SIZE],
            vec![0u8; BUFFER_CHUNK * BATCH_SIZE],
        ];
        let mut current_buf = 0;
        let mut messages_consumed = 0;
        let mut messages_processed = 0;
        let mut processing_errors = 0;

        loop {
            if *consumer_shutdown.borrow() {
                println!("DEBUG: Consumer shutting down");
                break;
            }

            let mut batch_size = 0;
            let buf = &mut msg_bufs[current_buf];

            for _ in 0..BATCH_SIZE {
                match consumer_ring.try_read(&mut buf[batch_size..]) {
                    Ok(size) => {
                        match ProcessedMessage::from_bytes(&buf[batch_size..batch_size + size]) {
                            Some(msg) => {
                                messages_processed += 1;
                                black_box(msg);
                            }
                            None => {
                                processing_errors += 1;
                            }
                        }
                        batch_size += size;
                        messages_consumed += 1;
                    }
                    Err(BrokerError::BufferEmpty) => {
                        if batch_size > 0 {
                            break;
                        }
                        tokio::task::yield_now().await;
                        continue;
                    }
                    Err(e) => {
                        eprintln!("ring buffer read error: {:?}", e);
                        break;
                    }
                }
            }

            if messages_consumed % 1_000_000 == 0 && messages_consumed > 0 {
                println!(
                    "Stats: consumed={}, processed={}, errors={}",
                    messages_consumed, messages_processed, processing_errors
                );
            }
            current_buf = 1 - current_buf;
        }
    });

    let mut header_buf = [0u8; std::mem::size_of::<MessageHeader>()];
    let mut msg_buf = vec![0u8; BUFFER_CHUNK * BATCH_SIZE];

    loop {
        if *shutdown.borrow() {
            println!("DEBUG: Connection handler shutting down");
            break;
        }

        tokio::select! {
            result = socket.read_exact(&mut header_buf) => {
                match result {
                    Ok(_) => {
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
                        }
                    }
                    Err(_) => break,
                }
            }
            _ = shutdown.changed() => {
                println!("DEBUG: handler received shutdown signal");
                break;
            }
        }
    }

    // instead of abort
    // let the consumer exit nicely before starting another
    consumer.await?;
    Ok(())
}
