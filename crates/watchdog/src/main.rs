use anyhow::Result;
use std::time::Duration;
use tokio::{
    signal::unix::{signal, SignalKind},
    sync::broadcast,
    time::sleep,
};
use tracing::{debug, info};
use tracing_appender::{
    non_blocking::WorkerGuard,
    rolling::{RollingFileAppender, Rotation},
};
use tracing_subscriber::{
    fmt::{self},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter, Layer, Registry,
};
use watchdog::{config, receiver::TxReceiver};
use zmq::Context;

const GRACEFUL_SHUTDOWN_TIMEOUT: u64 = 30;

#[tokio::main]
async fn main() -> Result<()> {
    // TIPS: guard must have same long lifetime with main
    let _guard = logger_init();

    let (tx, _rx) = broadcast::channel(1);
    let mut sigterm = signal(SignalKind::terminate())?;
    let mut sigint = signal(SignalKind::interrupt())?;
    let cfg = config::read_config();
    let context = Context::new();
    let subscriber = context.socket(zmq::SUB).unwrap();
    let zmq_url = format!("tcp://{}:{}", cfg.bitcoin.zmq, cfg.bitcoin.zmq_port);
    subscriber.connect(zmq_url.as_str()).unwrap();
    subscriber.set_subscribe(b"rawtx").unwrap();
    let tx_receiver = TxReceiver::new(cfg).await;

    info!("Start watchdog...");
    loop {
        tokio::select! {
            _ = sleep(Duration::from_millis(1)) => {
                let topic = subscriber.recv_msg(0).unwrap();
                debug!("Received topic : {:?}",topic.as_str());
                if topic.as_str().is_none(){
                    continue
                }

                if topic.as_str().unwrap() != "rawtx"{
                    continue
                }

                let tx_data = subscriber.recv_bytes(0).unwrap();
                tx_receiver.handle_recv(tx_data).await;
            }
            _ = sigterm.recv() => {
                info!("Received SIGTERM, shutting down gracefully...");
                break
            }
            _ = sigint.recv() => {
                info!("Received SIGINT, shutting down gracefully...");
                break
            }
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

    if !is_all_request_completed() {
        info!("Graceful shutdown timeout, closing server...");
    }

    info!("Close watchdog...");

    Ok(())
}

fn logger_init() -> WorkerGuard {
    let formatting_layer = fmt::layer().pretty().with_writer(std::io::stdout);
    let file_appender = RollingFileAppender::new(Rotation::HOURLY, "logs/watchdog", "watchdog.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
    let file_layer = fmt::layer()
        .with_ansi(false)
        .with_writer(non_blocking)
        // .with_writer(non_blocking.and(std::io::stdout))
        .with_filter(tracing_subscriber::filter::LevelFilter::DEBUG)
        .boxed();

    Registry::default()
        .with(formatting_layer)
        .with(file_layer)
        .with(EnvFilter::from_default_env())
        .init();

    guard
}

fn is_all_request_completed() -> bool {
    true
}
