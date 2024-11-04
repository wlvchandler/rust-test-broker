use tokio::net::TcpStream;
use tokio::io::{AsyncWriteExt, BufWriter};
use crate::{BUFFER_CHUNK, BATCH_SIZE};
use crate::error::NetworkError;
use crate::net::message::MessageHeader;

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

