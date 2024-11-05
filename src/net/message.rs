use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;
use std::hint::black_box;

pub struct ProcessedMessage {
    pub timestamp: u64,
    pub sequence: u64,
    pub checksum: u32,
    pub payload: Vec<u8>,
}

impl ProcessedMessage {
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < 20 {
            return None;
        }

        let timestamp = u64::from_le_bytes(data[0..8].try_into().ok()?);
        let sequence = u64::from_le_bytes(data[8..16].try_into().ok()?);
        let checksum = u32::from_le_bytes(data[16..20].try_into().ok()?);

        // validate checksum
        let mut hasher = DefaultHasher::new();
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

    pub fn process(&self) -> bool {
        // verify message is recent (~1s)
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;

        let age_nanos = now - self.timestamp;
        if age_nanos > 1_000_000_000 {
            return false;
        }

        // simulate payload processing
        let sum: u32 = self
            .payload
            .iter()
            .enumerate()
            .map(|(i, &b)| b as u32 * i as u32)
            .sum();
        black_box(sum);

        true
    }
}

#[repr(C, packed)]
pub struct MessageHeader {
    pub size: u32,
    pub batch_size: u32,
}

impl MessageHeader {
    #[inline(always)]
    pub fn size(&self) -> u32 {
        unsafe {
            let ptr = (self as *const MessageHeader).cast::<u8>();
            std::ptr::read_unaligned(ptr.cast::<u32>())
        }
    }

    #[inline(always)]
    pub fn batch_size(&self) -> u32 {
        unsafe {
            let ptr = (self as *const MessageHeader).cast::<u8>();
            std::ptr::read_unaligned(ptr.add(4).cast::<u32>())
        }
    }
}
