use anyhow::Result;
use std::time::Duration;
use tokio::{
    signal::unix::{signal, SignalKind},
    sync::broadcast,
    time::sleep,
};
use tracing::{error, info, warn};
use tracing_appender::{
    non_blocking::WorkerGuard,
    rolling::{RollingFileAppender, Rotation},
};
use tracing_subscriber::{
    fmt::{self, writer::MakeWriterExt},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter, Layer, Registry,
};
use watchdog::{
    btcrpc::{self, BtcCli},
    checker::Checker,
    config, receiver,
};

const GRACEFUL_SHUTDOWN_TIMEOUT: u64 = 30;

#[tokio::main]
async fn main() -> Result<()> {
    // TIPS: guard must have same long lifetime with main
    let _guard = logger_init();

    let (tx, rx) = broadcast::channel(1);
    let mut sigterm = signal(SignalKind::terminate())?;
    let mut sigint = signal(SignalKind::interrupt())?;

    let cfg = config::read_config();
    let btccli = BtcCli::new(&cfg.bitcoin.endpoint, &cfg.bitcoin.user, &cfg.bitcoin.pass);
    let checker = Checker::new(btccli);
    let handle = tokio::spawn(async move {
        receiver::receive_rawtx(rx, cfg, &checker).await;
    });

    tokio::select! {
        _ = sigterm.recv() => {
            info!("Received SIGTERM, shutting down gracefully...");
        }
        _ = sigint.recv() => {
            info!("Received SIGINT, shutting down gracefully...");
        }
    }

    tx.send(true).unwrap();
    let start_time = std::time::Instant::now();
    while start_time.elapsed().as_secs() < GRACEFUL_SHUTDOWN_TIMEOUT {
        if is_all_request_completed() {
            break;
        }
        sleep(Duration::from_secs(1)).await;
    }

    handle.await.unwrap();
    if !is_all_request_completed() {
        println!("Graceful shutdown timeout, closing server...");
    }

    Ok(())
}

fn logger_init() -> WorkerGuard {
    let formatting_layer = fmt::layer().pretty().with_writer(std::io::stdout);
    let file_appender = RollingFileAppender::new(Rotation::HOURLY, "logs/watchdog", "watchdog.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
    let file_layer = fmt::layer()
        .with_ansi(false)
        .with_writer(non_blocking.and(std::io::stdout))
        .with_filter(tracing_subscriber::filter::LevelFilter::INFO)
        .boxed();

    Registry::default()
        .with(formatting_layer)
        .with(file_layer)
        .with(EnvFilter::from_default_env())
        .init();

    guard
}

fn is_all_request_completed() -> bool {
    // 判断是否所有请求都已经处理完成
    true
}
