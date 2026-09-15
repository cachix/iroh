use std::{hint::black_box, sync::Arc, time::{Duration, Instant}};
use iroh::{SecretKey, endpoint::QuicTransportConfig, identity::{BuiltinAlgorithm, IdentityAlgorithm, IdentityEndpoint, LocalIdentity, Registry}};

fn identity(pq: bool, registry: &Registry) -> LocalIdentity {
    if pq { LocalIdentity::generate_ml_dsa65(registry).unwrap() }
    else { LocalIdentity::ed25519(SecretKey::generate(), registry).unwrap() }
}
fn suite(pq: bool) -> &'static str { if !pq { return "Ed25519"; } match std::env::var("BENCH_MIXED").as_deref() { Ok("client_pq") => "clientPQ-serverEd", Ok("server_pq") => "clientEd-serverPQ", _ => "ML-DSA-65" } }
fn cpu_ns() -> u64 {
    let mut t = libc::timespec { tv_sec: 0, tv_nsec: 0 };
    assert_eq!(unsafe { libc::clock_gettime(libc::CLOCK_THREAD_CPUTIME_ID, &mut t) }, 0);
    t.tv_sec as u64 * 1_000_000_000 + t.tv_nsec as u64
}
fn cpu() {
    let registry = Registry::builtins(vec![1,2]).unwrap();
    let message = [0x42; 130]; // TLS CertificateVerify-sized input.
    let ids = [identity(false, &registry), identity(true, &registry)];
    println!("suite,operation,batch,iterations,cpu_ns_per_op");
    for batch in 0..33 {
        for pq in if batch % 2 == 0 { [false,true] } else { [true,false] } {
            let id = &ids[pq as usize];
            let alg = if pq { BuiltinAlgorithm::MlDsa65 } else { BuiltinAlgorithm::Ed25519 };
            let spki = id.public_key();
            let raw = alg.public_key(spki.as_ref()).unwrap();
            let sig = id.sign(&message).unwrap();
            for operation in ["keygen", "sign", "verify", "verify_proof"] {
                let start = cpu_ns();
                for _ in 0..100 {
                    match operation {
                        "keygen" => { black_box(identity(pq, &registry)); }
                        "sign" => { black_box(id.sign(black_box(&message)).unwrap()); }
                        "verify" => { alg.verify(black_box(raw), black_box(&message), black_box(&sig)).unwrap(); }
                        _ => { black_box(registry.verify_proof(black_box(spki.as_ref()), black_box(&message), black_box(&sig)).unwrap()); }
                    }
                }
                let elapsed = cpu_ns() - start;
                if batch >= 3 { println!("{},{},{},100,{:.2}", suite(pq), operation, batch-3, elapsed as f64 / 100.0); }
            }
        }
    }
}
async fn network(samples: usize) {
    const ALPN: &[u8] = b"identity-bench/1";
    let registry = Arc::new(Registry::builtins(vec![1,2]).unwrap());
    let bulk_bytes = Arc::new(std::sync::atomic::AtomicU64::new(0));
    let mut bulk_endpoints = Vec::new();
    let mut bulk_tasks = Vec::new();
    if std::env::var_os("BENCH_COMPETE").is_some() {
        for _ in 0..2 {
            bulk_endpoints.push(IdentityEndpoint::builder(identity(false, &registry), registry.clone())
                .bind_addr("127.0.0.1:0".parse().unwrap())
                .alpns(vec![ALPN.to_vec()]).bind().await.unwrap());
        }
        let (outgoing, incoming) = tokio::join!(
            bulk_endpoints[0].connect(bulk_endpoints[1].addr().unwrap(), ALPN),
            bulk_endpoints[1].accept());
        let outgoing = outgoing.unwrap();
        let incoming = incoming.unwrap();
        bulk_tasks.push(tokio::spawn(async move {
            let mut stream = outgoing.open_uni().await.unwrap();
            loop { stream.write_all(&[0x6b; 16384]).await.unwrap(); }
        }));
        let count = bulk_bytes.clone();
        bulk_tasks.push(tokio::spawn(async move {
            let mut stream = incoming.accept_uni().await.unwrap();
            let mut buffer = [0; 16384];
            loop {
                let len = stream.read(&mut buffer).await.unwrap().expect("bulk stream ended");
                assert!(buffer[..len].iter().all(|&v| v == 0x6b));
                count.fetch_add(len as u64, std::sync::atomic::Ordering::Relaxed);
            }
        }));
        tokio::time::sleep(Duration::from_secs(3)).await;
        assert!(bulk_bytes.load(std::sync::atomic::Ordering::Relaxed) > 0);
    }
    let mut pairs = Vec::new();
    for pq in [false,true] {
        let mut endpoints = Vec::new();
        for role in 0..2 {
            let role_pq = pq && match std::env::var("BENCH_MIXED").as_deref() { Ok("client_pq") => role == 0, Ok("server_pq") => role == 1, _ => true };
            endpoints.push(IdentityEndpoint::builder(identity(role_pq, &registry), registry.clone())
                .bind_addr("127.0.0.1:0".parse().unwrap())
                .alpns(vec![ALPN.to_vec()])
                .transport_config(QuicTransportConfig::builder()
                    .max_idle_timeout(Some(Duration::from_secs(5).try_into().unwrap())).build())
                .bind().await.unwrap());
        }
        pairs.push(endpoints);
    }
    println!("suite,sample,handshake_us,client_tx_bytes,server_tx_bytes,status");
    let warmups = if samples == 0 || std::env::var_os("TRACE_BENCH").is_some() { 0 } else { 5 };
    let mut measured_start = Instant::now();
    let mut measured_bytes = bulk_bytes.load(std::sync::atomic::Ordering::Relaxed);
    for sample in 0..samples+warmups {
        if sample == warmups {
            measured_start = Instant::now();
            measured_bytes = bulk_bytes.load(std::sync::atomic::Ordering::Relaxed);
        }
        for pq in if sample % 2 == 0 { [false,true] } else { [true,false] } {
            let client = &pairs[pq as usize][0];
            let server = &pairs[pq as usize][1];
            if std::env::var_os("TRACE_BENCH").is_some() { eprintln!("BENCH_START suite={} client={} server={}", suite(pq), client.local_addr().unwrap(), server.local_addr().unwrap()); }
            let start = Instant::now();
            let result = tokio::time::timeout(Duration::from_secs(10), async {
                let (outgoing,incoming) = tokio::join!(client.connect(server.addr().unwrap(), ALPN), server.accept());
                match (outgoing,incoming) {
                    (Ok(outgoing),Ok(incoming)) => Ok((outgoing,incoming)),
                    errors => Err(format!("{errors:?}")),
                }
            }).await;
            let elapsed = start.elapsed().as_micros();
            if std::env::var_os("TRACE_BENCH").is_some() { eprintln!("BENCH_END suite={} elapsed_us={elapsed}", suite(pq)); }
            match result {
                Ok(Ok((outgoing,incoming))) => {
                    assert_eq!(outgoing.remote_id(),server.id());
                    assert_eq!(incoming.remote_id(),client.id());
                    if sample >= warmups {
                        println!("{},{},{},{},{},ok",suite(pq),sample-warmups,elapsed,outgoing.stats().udp_tx.bytes,incoming.stats().udp_tx.bytes);
                    }
                    if std::env::var_os("BENCH_VALIDATE_DATA").is_some() {
                        tokio::time::timeout(Duration::from_secs(10), async {
                            let (sent, received) = tokio::join!(async {
                                let mut stream = outgoing.open_uni().await.unwrap();
                                stream.write_all(&[0x5a; 1024]).await.unwrap();
                                stream.finish().unwrap();
                            }, async {
                                let mut stream = incoming.accept_uni().await.unwrap();
                                assert_eq!(stream.read_to_end(1024).await.unwrap(), [0x5a; 1024]);
                            });
                            let _ = (sent, received);
                        }).await.expect("application data exchange timed out");
                    }
                    outgoing.close(0u32.into(),b"sample complete");
                    incoming.close(0u32.into(),b"sample complete");
                }
                failed => {
                    eprintln!("handshake failed: {failed:?}");
                    if sample >= warmups { println!("{},{},{},0,0,failed",suite(pq),sample-warmups,elapsed); }
                }
            }
            // Let close packets clear the emulated path before the next sample.
            tokio::time::sleep(Duration::from_millis(120)).await;
            if let Ok(interval) = std::env::var("BENCH_INTERVAL_MS") {
                tokio::time::sleep_until(tokio::time::Instant::from_std(start + Duration::from_millis(interval.parse().unwrap()))).await;
            }
        }
    }
    if samples == 0 { tokio::time::sleep(Duration::from_secs(20)).await; }
    let elapsed = measured_start.elapsed().as_secs_f64();
    let received = bulk_bytes.load(std::sync::atomic::Ordering::Relaxed) - measured_bytes;
    eprintln!("BULK_RESULT seconds={elapsed:.6} bytes={received} mbps={:.6} handshakes_per_second={:.6}", received as f64 * 8.0 / elapsed / 1e6, samples as f64 * 2.0 / elapsed);
    for task in &bulk_tasks { assert!(!task.is_finished(), "bulk transfer stopped early"); }
    for task in bulk_tasks { task.abort(); let _ = task.await; }
    for endpoint in &bulk_endpoints { endpoint.close().await; }
    for pair in &pairs { for endpoint in pair { endpoint.close().await; } }
}
#[tokio::main]
async fn main() {
    if std::env::var_os("TRACE_BENCH").is_some() {
        tracing_subscriber::fmt().with_env_filter(tracing_subscriber::EnvFilter::from_default_env()).with_ansi(false).with_writer(std::io::stderr).init();
    }
    let args: Vec<_> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("cpu") { cpu(); }
    else { network(args.get(2).map(|s| s.parse().unwrap()).unwrap_or(100)).await; }
}
