use crate::{checker::sign::SignChecker, config, lightning};
pub use anyhow::Result;
use bitcoin::{consensus::deserialize, Transaction};
use std::time::Duration;
use tgbot::TgBot;
use tokio::{sync::broadcast::Receiver, time::sleep};
use tracing::{error, info, warn};
use zmq::Context;

#[tracing::instrument(skip_all)]
pub async fn receive_rawtx(
    mut stop_sig: Receiver<bool>,
    cfg: config::Config,
    checker: &SignChecker,
) {
    let context = Context::new();
    let subscriber = context.socket(zmq::SUB).unwrap();
    subscriber
        .connect(format!("tcp://{}:{}", cfg.bitcoin.zmq, cfg.bitcoin.zmq_port).as_str())
        .unwrap();
    subscriber.set_subscribe(b"rawtx").unwrap();
    info!("Subscribed to raw transactions...");
    let bot = TgBot::new(&cfg.tgbot.token, cfg.tgbot.chat_id, cfg.tgbot.tx_topic_id);
    loop {
        tokio::select! {
            _ = stop_sig.recv() => {
                println!("Received exit signal, breaking the loop.");
                break;
            }
            _ = sleep(Duration::from_millis(1)) => {
                let _topic = subscriber.recv_msg(0).unwrap();
                let tx_data = subscriber.recv_bytes(0).unwrap();
                match deserialize::<Transaction>(&tx_data) {
                    Ok(tx) => {
                        handle_tx(tx, &bot,checker).await;
                    }
                    Err(_) => {
                        // eprintln!("Failed to deserialize transaction: {}", e);
                    }
                }
            }
        }
    }
}

async fn handle_tx(tx: Transaction, bot: &TgBot, checker: &SignChecker) {
    let txid = tx.compute_txid();
    let mut exist = false;
    let mut input_idx = 0;
    for (idx, input) in tx.input.iter().enumerate() {
        if input.witness.len() <= 0 {
            continue;
        }

        if input.witness[0].len() >= 32 {
            continue;
        }

        if checker.check_input_sign(&input) {
            continue;
        }

        if input.witness.len() >= 1
            && lightning::is_swept_lightning_anchor(&hex::encode(&input.witness[1]))
        {
            warn!(
                "Received transaction hash: {}. Swept Lightning Anchor",
                txid
            );
            continue;
        }

        exist = true;
        input_idx = idx;
        break;
    }

    if exist {
        info!("Received transaction hash: {}, idx : {}", txid, input_idx);
        let msg = format!("txid:{},idx:{}", txid.to_string(), input_idx);
        match bot.send_msg_to_topic(msg.as_str()).await {
            Ok(_) => {}
            Err(e) => error!("send msg to tg failed. {}", e),
        };
    }
}
