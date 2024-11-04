use broker::{RingBuffer, Metrics};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use std::io::Write;

const ITERATIONS: usize = 1_000_000;
const WARMUP_ITERATIONS: usize = 10_000;
const MAX_MESSAGE_SIZE: usize = 8192;

/// Runs a single benchmark iteration for a given message size
///
fn run_benchmark(message_size: usize) -> Metrics {
    assert!(message_size <= MAX_MESSAGE_SIZE, "Message size too large");
    
    let ring = Arc::new(RingBuffer::new().expect("Failed to create ring buffer"));
    let ring_consumer = ring.clone();

    // prealloc buffers BEFORE spawning threads
    let message = vec![1u8; message_size];
    let read_buffer = vec![0u8; message_size];

    let producer = thread::spawn({
        let message = message.clone();
        move || {
            let mut latencies = Vec::with_capacity(ITERATIONS);
            
            // foreplay
            //
            println!("  Warming up...");
            for _ in 0..WARMUP_ITERATIONS {
                while ring.try_write(&message).is_err() {
                    core::hint::spin_loop();
                }
            }
            
            // main benchmark loop
            //
            println!("  Running main benchmark...");
            let main_start = Instant::now();
            
            for i in 0..ITERATIONS {
                if i % (ITERATIONS / 10) == 0 {
                    print!("  {}%...", (i * 100) / ITERATIONS);
                    let _ = std::io::stdout().flush();
                }
                
                let start = Instant::now();
                while ring.try_write(&message).is_err() {
                    core::hint::spin_loop();
                }
                latencies.push(start.elapsed());
            }
            println!("100%");
            
            Metrics::from_measurements(
                &mut latencies,
                message_size * ITERATIONS,
                main_start.elapsed()
            )
        }
    });

    let consumer = thread::spawn({
        let mut read_buffer = read_buffer;
        move || {
            let mut count = 0;
            while count < (ITERATIONS + WARMUP_ITERATIONS) {
                match ring_consumer.try_read(&mut read_buffer) {
                    Ok(_) => count += 1,
                    Err(_) => core::hint::spin_loop(),
                }
            }
        }
    });

    let result = producer.join().unwrap();
    consumer.join().unwrap();
    result
}

fn main() {
    println!("--------------------------------");
    println!("Iterations per size: {}", ITERATIONS);
    println!("Warmup iterations: {}", WARMUP_ITERATIONS);
    println!("--------------------------------\n");
    
    // Test different message sizes
    let sizes = [
        32,    // baseline
        64,    // cache line size
        128,   // double cache line
        256,   // quad cache line
        512,   // half kb
        1024,  // 1kb
        4096,  // page size
    ];
    
    for &size in &sizes {
        println!("\nBenchmarking message size: {} bytes", size);
        println!("--------------------------------");
        
        let result = run_benchmark(size);
        
        println!("\nResults for {} bytes:", size);
        println!("Latency Statistics:");
        println!("  min: {:?}", result.min);
        println!("  p50: {:?}", result.p50);
        println!("  p99: {:?}", result.p99);
        println!("  p99.9: {:?}", result.p99_9);
        println!("  max: {:?}", result.max);
        println!("Throughput:");
        println!("  Messages/sec: {:.2}", result.msgs_per_sec);
        println!("  MB/sec: {:.2}", result.mb_per_sec);
        println!("  Gb/sec: {:.2}", result.mb_per_sec / 1000.0);
        
        // let the system settle a little between tests
        thread::sleep(Duration::from_millis(666));
    }
}
