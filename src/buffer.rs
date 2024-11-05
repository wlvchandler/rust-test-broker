use std::sync::atomic::{AtomicU64, Ordering};
use std::ptr;
use std::alloc::{self, Layout};

use crate::error::BrokerError;
use crate::{CACHE_LINE_SIZE, RING_BUFFER_SIZE};

#[repr(C, align(64))]
pub struct RingBuffer {
    data: *mut u8,
    mask: usize,
    producer_index: AtomicU64,
    consumer_index: AtomicU64,
    _pad: [u8; CACHE_LINE_SIZE - 32],
}

unsafe impl Send for RingBuffer {}
unsafe impl Sync for RingBuffer {}

impl RingBuffer {
    pub fn new() -> Result<Self, BrokerError> {
        let layout = Layout::from_size_align(RING_BUFFER_SIZE, CACHE_LINE_SIZE)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        
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

    #[inline(always)]
    pub fn try_write(&self, data: &[u8]) -> Result<(), BrokerError> {
        let size = data.len();
        if size > (self.mask + 1) / 4 {
            return Err(BrokerError::MessageTooLarge);
        }
        
        let producer_index = self.producer_index.load(Ordering::Relaxed);
        let consumer_index = self.consumer_index.load(Ordering::Acquire);
        
        if producer_index.wrapping_sub(consumer_index) > (self.mask as u64 - size as u64) {
            return Err(BrokerError::BufferFull);
        }

        let write_index = (producer_index as usize) & self.mask;
        let buffer_end = self.mask + 1;
        let first_part = buffer_end - write_index;
        if size <= first_part {
            unsafe {
                ptr::copy_nonoverlapping(
                    data.as_ptr(),
                    self.data.add(write_index),
                    size,
                );
            }
        } else {
            unsafe {
                ptr::copy_nonoverlapping(
                    data.as_ptr(),
                    self.data.add(write_index),
                    first_part,
                );
                ptr::copy_nonoverlapping(
                    data.as_ptr().add(first_part),
                    self.data,
                    size-first_part,
                );
            }
        }

        self.producer_index.store(
            producer_index.wrapping_add(size as u64),
            Ordering::Release,
        );
        Ok(())
    }

    #[inline(always)]
    pub fn try_read(&self, buf: &mut [u8]) -> Result<usize, BrokerError> {
        let consumer_index = self.consumer_index.load(Ordering::Relaxed);
        let producer_index = self.producer_index.load(Ordering::Acquire);

        if consumer_index == producer_index {
            return Err(BrokerError::BufferEmpty);
        }

        let available = producer_index.wrapping_sub(consumer_index) as usize;
        let size = buf.len().min(available);
        let read_index = (consumer_index as usize) & self.mask;
        let buffer_end = self.mask + 1;
        let first_part = buffer_end-read_index;

        if size <= first_part {
            unsafe {
                ptr::copy_nonoverlapping(
                    self.data.add(read_index),
                    buf.as_mut_ptr(),
                    size,
                );
            }
        } else { // wrap around
            unsafe {
                ptr::copy_nonoverlapping(
                    self.data.add(read_index),
                    buf.as_mut_ptr(),
                    first_part,
                );
                ptr::copy_nonoverlapping(
                    self.data,
                    buf.as_mut_ptr().add(first_part),
                    size - first_part,
                );
            }
        }

        self.consumer_index.store(
            consumer_index.wrapping_add(size as u64),
            Ordering::Release,
        );

        Ok(size)
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
