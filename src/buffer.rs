use std::sync::atomic::{AtomicU64, Ordering};
use std::ptr;
use std::alloc::{self, Layout};

use crate::{CACHE_LINE_SIZE, RING_BUFFER_SIZE, MESSAGE_HEADER_SIZE};
use crate::error::BrokerError;

///note: the header to each message in the ring buffer, aligned to prevent false sharing
///
#[repr(C, align(64))]
struct MessageHeader {
    sequence: u64,    
    size: u32,       
    flags: u32,  // e.g., message types, priorities, blah blah. not used yet
}

/// lock-free ring buffer. cache-line alignment prevents false sharing between cores
///
#[repr(C, align(64))]
pub struct RingBuffer {
    data: *mut u8,                 
    mask: usize,    // mask for quick modulo
    producer_index: AtomicU64,      
    consumer_index: AtomicU64,       
    _pad: [u8; CACHE_LINE_SIZE - 32], 
}

// for  sending the buffer between threads...
unsafe impl Send for RingBuffer {}
unsafe impl Sync for RingBuffer {}

impl RingBuffer {
    ///  create a new ring buffer with zero-allocated memory
    pub fn new() -> Result<Self, BrokerError> {
        // Ensure power of 2 size for efficient masking
        let layout = Layout::from_size_align(RING_BUFFER_SIZE, CACHE_LINE_SIZE)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        
        // alloc zeroed mem for predictable behavior
        let data = unsafe { alloc::alloc_zeroed(layout) };
        if data.is_null() {
            return Err(BrokerError::SystemError(
                std::io::Error::new(std::io::ErrorKind::Other, "Memory allocation failed")
            ));
        }

        Ok(RingBuffer {
            data,
            mask: RING_BUFFER_SIZE - 1,
            producer_index: AtomicU64::new(0),
            consumer_index: AtomicU64::new(0),
            _pad: [0; CACHE_LINE_SIZE - 32],
        })
    }

    /// Attempt to write a message to the ring buffer.
    /// (relaxed/release memory ordering)
    ///
    #[inline(always)]
    pub fn try_write(&self, data: &[u8]) -> Result<(), BrokerError> {
        let size = data.len();
        
        // buffer overflow cehck
        if size > (self.mask + 1) / 4 {
            return Err(BrokerError::MessageTooLarge);
        }
        let total_size = size + MESSAGE_HEADER_SIZE;
        let producer_index = self.producer_index.load(Ordering::Relaxed);
        let consumer_index = self.consumer_index.load(Ordering::Acquire);
        
        // check if we have enough space
        if producer_index.wrapping_sub(consumer_index) > (self.mask as u64 - total_size as u64) {
            return Err(BrokerError::BufferFull);
        }

        let write_index = (producer_index as usize) & self.mask;
        
        // write header and data in one go
        unsafe {
            ptr::write(
                self.data.add(write_index) as *mut MessageHeader,
                MessageHeader {
                    sequence: producer_index,
                    size: size as u32,
                    flags: 0,
                }
            );
            
            ptr::copy_nonoverlapping(
                data.as_ptr(),
                self.data.add(write_index + MESSAGE_HEADER_SIZE),
                size,
            );
        }

        // show to consumers
        self.producer_index.store(
            producer_index.wrapping_add(total_size as u64),
            Ordering::Release,
        );
        Ok(())
    }

    /// read a message from the ring buffer, return the number of bytes read
    ///
    #[inline(always)]
    pub fn try_read(&self, buf: &mut [u8]) -> Result<usize, BrokerError> {
        let consumer_index = self.consumer_index.load(Ordering::Relaxed);
        let producer_index = self.producer_index.load(Ordering::Acquire);

        if consumer_index == producer_index {
            return Err(BrokerError::BufferFull);
        }

        let read_index = (consumer_index as usize) & self.mask;
        
        let header = unsafe {
            ptr::read(self.data.add(read_index) as *const MessageHeader)
        };

        // make sure output buffer is large enough
        if buf.len() < header.size as usize {
            return Err(BrokerError::BufferTooSmall);
        }

        // copy to output buffer
        unsafe {
            ptr::copy_nonoverlapping(
                self.data.add(read_index + MESSAGE_HEADER_SIZE),
                buf.as_mut_ptr(),
                header.size as usize,
            );
        }

        self.consumer_index.store(
            consumer_index.wrapping_add((MESSAGE_HEADER_SIZE + header.size as usize) as u64),
            Ordering::Release,
        );

        Ok(header.size as usize)
    }
}

impl Drop for RingBuffer {
    fn drop(&mut self) {
        unsafe {
            let layout = Layout::from_size_align_unchecked(RING_BUFFER_SIZE, CACHE_LINE_SIZE);
            alloc::dealloc(self.data, layout);
        }
    }
}