use anyhow::Result;
use std::{sync::Arc, time::Duration};

use tokio::{
    signal::unix::{signal, SignalKind},
    sync::{broadcast, RwLock},
    time::sleep,
};

use tracing::{debug, error, info};
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
use watchdog::{config, receiver::TxReceiver, sender::tx::TxSender, syncer::Syncer, utxo};
use zmq::Context;

#[tokio::main]
async fn main() -> Result<()> {
    // TIPS: guard must have same long lifetime with main
    let _guard = logger_init();

    let (tx, mut rx) = broadcast::channel(1);
    let (tx_send, mut tx_msg_rcv) = broadcast::channel(1024);
    let mut sigterm = signal(SignalKind::terminate())?;
    let mut sigint = signal(SignalKind::interrupt())?;
    let cfg = config::read_config();
    let context = Context::new();
    let subscriber = context.socket(zmq::SUB).unwrap();
    let zmq_url = format!("tcp://{}:{}", cfg.bitcoin.zmq, cfg.bitcoin.zmq_port);
    subscriber.connect(zmq_url.as_str()).unwrap();
    subscriber.set_subscribe(b"rawtx").unwrap();

    let shared_data = Arc::new(RwLock::new(Vec::new()));
    let shared_data2 = shared_data.clone();
    let utxo_updater = utxo::UtxoUpdater::new(&cfg, shared_data2);
    let utxo_update_task = tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = sleep(Duration::from_secs(30)) => {
                    match utxo_updater.update_utxo().await{
                        Ok(_) => {}
                        Err(e) => {
                            error!("utxo_update_task failed {}", e);
                        }
                    }
                }
                _ = rx.recv() => {
                    info!("Received SIGTERM, utxo_update_task shutting down gracefully...");
                    return;
                }
            }
        }
    });

    let tx_receiver = TxReceiver::new(&cfg).await;
    let mut rx1 = tx.subscribe();
    let tx_send1 = tx_send.clone();
    let receiver_task = tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = sleep(Duration::from_millis(10)) => {
                    let topic = subscriber.recv_msg(0).unwrap();
                    debug!("Received topic : {:?}", topic.as_str());
                    if topic.as_str().is_none() {
                        continue;
                    }

                    if topic.as_str().unwrap() != "rawtx" {
                        continue;
                    }

                    let tx_data = subscriber.recv_bytes(0).unwrap();
                    let sender1 = tx_send1.clone();

                    match tx_receiver.handle_recv(tx_data,sender1).await{
                        Ok(_) => {}
                        Err(e) => {
                            error!("handle tx receiver {}", e);
                        }
                    }
                }
                _ = rx1.recv() => {
                    info!("Received SIGTERM, receiver task shutting down gracefully...");
                    return;
                }
            }
        }
    });

    let anchor_syncer = Syncer::new(&cfg).await;
    let mut rx2 = tx.subscribe();
    let syncer_task = tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = sleep(Duration::from_secs(10)) => {
                    info!("Start Syncer ...");
                    anchor_syncer.sync_anchor().await;
                }
                _ = rx2.recv() => {
                    info!("Received SIGTERM, sync task shutting down gracefully...");
                    return;

                }
            }
        }
    });

    let tx_sender = TxSender::new(&cfg).await;
    let mut rx3 = tx.subscribe();
    let shared_data3 = Arc::clone(&shared_data);
    let sender_task = tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = sleep(Duration::from_secs(3)) => {
                    info!("Start Tx Sender Anchor...");
                    match tx_sender.send_task().await{
                        Ok(_)=> {},
                        Err(e) => {
                            error!("Error Sender Anchor: {:?}", e);
                        }
                    }
                }
                info = tx_msg_rcv.recv() => {
                    info!("Start Tx Sender Unsigned : {:?}", info);
                    let my_utxos = shared_data3.read().await;
                    let my_utxos = my_utxos.clone();
                    match info{
                        Ok((tx,idx))=> {
                            match tx_sender.send_unsigned_tx(tx, idx, my_utxos).await{
                                Ok(_)=> {},
                                Err(err) => {
                                    error!("Error Sender Unsigned task: {:?}", err);
                                }
                            }
                        },
                        Err(err) => {
                            error!("Error Sender Unsigned task: {:?}", err);
                        }
                    }
                }
                _ = rx3.recv() => {
                    info!("Received SIGTERM, tx send task shutting down gracefully...");
                    return;
                }
            }
        }
    });

    let stop_sig_task = tokio::spawn(async move {
        tokio::select! {
            _ = sigterm.recv() => {
                tx.send(true).unwrap();
                info!("Received SIGTERM, shutting down gracefully...");
            }
            _ = sigint.recv() => {
                tx.send(true).unwrap();
                info!("Received SIGINT, shutting down gracefully...");
            }
        }
    });

    info!("Start watchdog...");
    let _ = tokio::join!(
        receiver_task,
        syncer_task,
        sender_task,
        utxo_update_task,
        stop_sig_task
    );
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
