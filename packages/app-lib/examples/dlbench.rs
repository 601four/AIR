//! 下载速度自测
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use futures::StreamExt;

fn main() {
    let mut args = std::env::args().skip(1);
    let list_path = args.next().expect("urls.txt");
    let parallel: usize = args.next().unwrap_or_else(|| "32".into()).parse().unwrap();
    let timeout: u64 = args.next().unwrap_or_else(|| "30".into()).parse().unwrap();

    let content = std::fs::read_to_string(&list_path).expect("read list");
    let urls: Vec<String> = content
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| l.split_whitespace().next().unwrap().to_string())
        .collect();

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(8)
        .enable_all()
        .build()
        .unwrap();

    runtime.block_on(async move {
        let mut builder = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(15))
            .tcp_keepalive(Some(Duration::from_secs(10)))
            .read_timeout(Duration::from_secs(timeout))
            .user_agent("AirCore-bench/1.0");
        if std::env::var("HTTP1_ONLY").is_ok() {
            builder = builder.http1_only();
        }
        let client = builder.build().expect("client");

        let total = Arc::new(AtomicU64::new(0));
        let started = Instant::now();
        let client = Arc::new(client);
        let queue = Arc::new(tokio::sync::Mutex::new(urls.into_iter()));
        let mut handles = Vec::new();
        for _ in 0..parallel {
            let queue = queue.clone();
            let client = client.clone();
            let total = total.clone();
            handles.push(tokio::spawn(async move {
                loop {
                    let next = { queue.lock().await.next() };
                    let Some(url) = next else { break };
                    if let Ok(response) = client.get(&url).send().await {
                        if response.status().is_success() {
                            let mut stream = response.bytes_stream();
                            while let Some(Ok(chunk)) = stream.next().await {
                                total.fetch_add(chunk.len() as u64, Ordering::Relaxed);
                            }
                        }
                    }
                }
            }));
        }
        for handle in handles {
            let _ = handle.await;
        }
        let seconds = started.elapsed().as_secs_f64();
        let bytes = total.load(Ordering::Relaxed) as f64;
        println!(
            "RESULT parallel={parallel} bytes={:.2}MB time={:.2}s speed={:.2}MB/s",
            bytes / 1048576.0,
            seconds,
            bytes / 1048576.0 / seconds
        );
    });
}
