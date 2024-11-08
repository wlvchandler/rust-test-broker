### non networked, cache only, un-tuned system

| Message Size | Messages/sec | Throughput (Gb/sec) | Min Latency | P50 Latency | P99 Latency | P99.9 Latency | Max Latency |
|------------:|-------------:|-------------------:|------------:|------------:|------------:|---------------|------------:|
| 32 bytes    | 19,479,258  | 0.59              | 10ns        | 30ns        | 41ns        | 70ns          | 7.865µs     |
| 64 bytes    | 23,697,883  | 1.45              | 10ns        | 20ns        | 60ns        | 71ns          | 22.913µs    |
| 128 bytes   | 26,589,715  | 3.25              | 10ns        | 20ns        | 40ns        | 70ns          | 14.557µs    |
| 256 bytes   | 19,251,907  | 4.70              | 10ns        | 30ns        | 71ns        | 110ns         | 24.196µs    |
| 512 bytes   | 17,026,549  | 8.31              | 10ns        | 40ns        | 81ns        | 150ns         | 47.951µs    |
| 1024 bytes  | 10,886,592  | 10.63             | 10ns        | 41ns        | 160ns       | 361ns         | 50.806µs    |
| 4096 bytes  | 3,001,404   | 11.72             | 40ns        | 271ns       | 582ns       | 762ns         | 376.454µs   |

### non networked, cache only, tuned system

| Message Size | Messages/sec | Throughput (Gb/sec) | Min Latency | P50 Latency | P99 Latency | P99.9 Latency | Max Latency |
|------------:|-------------:|-------------------:|------------:|------------:|------------:|---------------|------------:|
| 32 bytes    | 19,776,911  | 0.60              | 10ns        | 30ns        | 50ns        | 71ns          | 83.017µs    |
| 64 bytes    | 24,369,761  | 1.49              | 10ns        | 20ns        | 50ns        | 71ns          | 48.321µs    |
| 128 bytes   | 24,632,338  | 3.01              | 10ns        | 20ns        | 41ns        | 70ns          | 46.017µs    |
| 256 bytes   | 18,853,202  | 4.60              | 10ns        | 30ns        | 80ns        | 100ns         | 19.136µs    |
| 512 bytes   | 17,388,410  | 8.49              | 10ns        | 30ns        | 80ns        | 180ns         | 17.232µs    |
| 1024 bytes  | 11,550,830  | 11.28             | 10ns        | 40ns        | 140ns       | 331ns         | 499.366µs   |
| 4096 bytes  | 3,438,658   | 13.43             | 40ns        | 241ns       | 561ns       | 702ns         | 61.767µs    |


### TCP (localhost), tuned system (Average of 15 runs)

| Message Size | Messages/s  | Throughput (Gbps) | 
|------------:|-------------:|-------------------:|
| 1024 bytes  | 4,109,253.19 | 33.68              |



#### optimizations?
- large batching (which means minimal syscalls)
- object refurbishing/buffer reuse
- zero-copy where possible
- carefully aligned memory access
- minimal overhead from actual data processing after the consumer eats it.
  - with a black box in the consumer task, it can get up to the throughput lol

```rust
// in consumer task we could just do this to prevent compiler optimization I believe
if batch_size > 0 {
    black_box(&msg_buf[..batch_size]);  
}
```

Worth noting that despite all the performance measures, this throughput/latency is just as importantly from a couple other things:
- we're localhost, so TCP messages are not actually hitting hardware. And, modern kernels heavily optimize localhost tcp, especially with TCP_NODELAY
- data stays in kernel memory and uses memory copies instead of network io

I plan to test later over a LAN. That being said, with properly tuned NICs capable of high Gbps similar numbers are possible. Or at least with some kind of kernel bypass :P Can't really do DPDK easily but maybe I'll try to do that later 


### how to run
``` cargo build --release ```

Open two terminals or run the server in the background

```
# terminal 1
cargo run --release --bin server

# terminal 2
cargo run --release --bin server
```

#### Example 

```
will@DESKTOP-71HHMI5:~/broker$ cargo run --release --bin client
   Compiling broker v0.1.0 (/home/rust/broker)
    Finished `release` profile [optimized] target(s) in 2.93s
     Running `target/release/client`
Starting client. Press Ctrl+C to stop.
test_start msg_bytes:1024
DEBUG: Connecting to 127.0.0.1:7878
SUCCESS: Connected to 127.0.0.1:7878
DEBUG: Client sent 16 million messages
DEBUG: Client sent 32 million messages
DEBUG: Client sent 48 million messages
DEBUG: Client sent 64 million messages
DEBUG: Client sent 80 million messages
DEBUG: Client sent 96 million messages
DEBUG: Client sent 112 million messages
DEBUG: Client sent 128 million messages
^C
Shutdown signal received. Stopping client...

Results (time: 36.602244707s):
Throughput: 3.66 GB/s
Messages/sec: 3574831.27
Total messages sent: 130846849
Total bytes sent: 133987173376
```


#### first round of latency benchmarking before throughput was prioritized (removed `bench.rs` for now):

```
will@DESKTOP-71HHMI5:~/broker$ ./target/release/bench
--------------------------------
Iterations per size: 1000000
Warmup iterations: 10000
--------------------------------
Benchmarking message size: 32 bytes
--------------------------------
  Warming up...
  Running main benchmark...
  0%...  10%...  20%...  30%...  40%...  50%...  60%...  70%...  80%...  90%...100%
Results for 32 bytes:
Latency Statistics:
  min: 10ns
  p50: 30ns
  p99: 50ns
  p99.9: 71ns
  max: 83.017µs
Throughput:
  Messages/sec: 19776911.69
  MB/sec: 603.54
  Gb/sec: 0.60
Benchmarking message size: 64 bytes
--------------------------------
  Warming up...
  Running main benchmark...
  0%...  10%...  20%...  30%...  40%...  50%...  60%...  70%...  80%...  90%...100%
Results for 64 bytes:
Latency Statistics:
  min: 10ns
  p50: 20ns
  p99: 50ns
  p99.9: 71ns
  max: 48.321µs
Throughput:
  Messages/sec: 24369761.42
  MB/sec: 1487.41
  Gb/sec: 1.49
Benchmarking message size: 128 bytes
--------------------------------
  Warming up...
  Running main benchmark...
  0%...  10%...  20%...  30%...  40%...  50%...  60%...  70%...  80%...  90%...100%
Results for 128 bytes:
Latency Statistics:
  min: 10ns
  p50: 20ns
  p99: 41ns
  p99.9: 70ns
  max: 46.017µs
Throughput:
  Messages/sec: 24632338.94
  MB/sec: 3006.88
  Gb/sec: 3.01
Benchmarking message size: 256 bytes
--------------------------------
  Warming up...
  Running main benchmark...
  0%...  10%...  20%...  30%...  40%...  50%...  60%...  70%...  80%...  90%...100%
Results for 256 bytes:
Latency Statistics:
  min: 10ns
  p50: 30ns
  p99: 80ns
  p99.9: 100ns
  max: 19.136µs
Throughput:
  Messages/sec: 18853202.67
  MB/sec: 4602.83
  Gb/sec: 4.60
Benchmarking message size: 512 bytes
--------------------------------
  Warming up...
  Running main benchmark...
  0%...  10%...  20%...  30%...  40%...  50%...  60%...  70%...  80%...  90%...100%
Results for 512 bytes:
Latency Statistics:
  min: 10ns
  p50: 30ns
  p99: 80ns
  p99.9: 180ns
  max: 17.232µs
Throughput:
  Messages/sec: 17388410.31
  MB/sec: 8490.43
  Gb/sec: 8.49
Benchmarking message size: 1024 bytes
--------------------------------
  Warming up...
  Running main benchmark...
  0%...  10%...  20%...  30%...  40%...  50%...  60%...  70%...  80%...  90%...100%
Results for 1024 bytes:
Latency Statistics:
  min: 10ns
  p50: 40ns
  p99: 140ns
  p99.9: 331ns
  max: 499.366µs
Throughput:
  Messages/sec: 11550830.83
  MB/sec: 11280.11
  Gb/sec: 11.28
Benchmarking message size: 4096 bytes
--------------------------------
  Warming up...
  Running main benchmark...
  0%...  10%...  20%...  30%...  40%...  50%...  60%...  70%...  80%...  90%...100%
Results for 4096 bytes:
Latency Statistics:
  min: 40ns
  p50: 241ns
  p99: 561ns
  p99.9: 702ns
  max: 61.767µs
Throughput:
  Messages/sec: 3438658.11
  MB/sec: 13432.26
  Gb/sec: 13.43
```
