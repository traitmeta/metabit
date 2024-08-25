use tracing::debug;

use super::*;
use crate::{
    btcrpc::BtcCli,
    checker::{lightning::LightningChecker, sign::SignChecker},
    config, lightning,
};

pub struct TxReceiver {
    // subscriber: Arc<Socket>,
    bot: Arc<TgBot>,
    sign_checker: Arc<SignChecker>,
    lightning_checker: Arc<LightningChecker>,
}

impl TxReceiver {
    pub fn new(cfg: config::Config) -> Self {
        let bot = TgBot::new(&cfg.tgbot.token, cfg.tgbot.chat_id, cfg.tgbot.tx_topic_id);
        let btccli = BtcCli::new(&cfg.bitcoin.endpoint, &cfg.bitcoin.user, &cfg.bitcoin.pass);
        let sign_checker = SignChecker::new(btccli);
        let lightning_checker = LightningChecker::new();
        Self {
            // subscriber,
            bot: Arc::new(bot),
            sign_checker: Arc::new(sign_checker),
            lightning_checker: Arc::new(lightning_checker),
        }
    }
    // pub fn new(cfg: config::Config) -> Self {
    //     let context = Context::new();
    //     let subscriber = context.socket(zmq::SUB).unwrap();
    //     subscriber
    //         .connect(format!("tcp://{}:{}", cfg.bitcoin.zmq, cfg.bitcoin.zmq_port).as_str())
    //         .unwrap();
    //     subscriber.set_subscribe(b"rawtx").unwrap();
    //     let bot = TgBot::new(&cfg.tgbot.token, cfg.tgbot.chat_id, cfg.tgbot.tx_topic_id);

    //     let btccli = BtcCli::new(&cfg.bitcoin.endpoint, &cfg.bitcoin.user, &cfg.bitcoin.pass);
    //     let sign_checker = SignChecker::new(btccli);
    //     let lightning_checker = LightningChecker::new();
    //     Self {
    //         subscriber,
    //         bot: Arc::new(bot),
    //         sign_checker: Arc::new(sign_checker),
    //         lightning_checker: Arc::new(lightning_checker),
    //     }
    // }

    #[tracing::instrument(skip_all)]
    pub async fn recv(&self, subscriber: Arc<Socket>, mut stop_sig: Receiver<bool>) {
        info!("Subscribed to raw transactions...");
        loop {
            tokio::select! {
                _ = stop_sig.recv() => {
                    println!("Received exit signal, breaking the loop.");
                    break;
                }
                _ = sleep(Duration::from_millis(1)) => {
                    // let _topic = self.subscriber.recv_msg(0).unwrap();
                    // let tx_data = self.subscriber.recv_bytes(0).unwrap();
                    let _topic = subscriber.recv_msg(0).unwrap();
                    let tx_data = subscriber.recv_bytes(0).unwrap();
                    match deserialize::<Transaction>(&tx_data) {
                        Ok(tx) => {
                            let my_bot = self.bot.clone();
                            let my_sign_checker = self.sign_checker.clone();
                            let tx1 = tx.clone();
                            let sign_handle = tokio::spawn(async {
                                handle_tx_thread(tx1, my_bot,my_sign_checker).await;
                            });

                            let my_lightning_checker = self.lightning_checker.clone();
                            let tx2 = tx.clone();
                            let my_bot2 = self.bot.clone();
                            let lightning_handle = tokio::spawn(async {
                                handle_tx_lightning(tx2, my_bot2,my_lightning_checker).await;
                            });

                            sign_handle.await.unwrap();
                            lightning_handle.await.unwrap();
                        }
                        Err(_) => {
                            // eprintln!("Failed to deserialize transaction: {}", e);
                        }
                    }
                }
            }
        }
    }

    #[tracing::instrument(skip_all)]
    pub async fn handle_recv(&self, tx_data: Vec<u8>) {
        if tx_data.len() == 0 {
            return;
        }

        debug!("received from zmq : {:?}", tx_data);
        match deserialize::<Transaction>(&tx_data) {
            Ok(tx) => {
                info!("received tx : {}", tx.compute_txid());
                let my_bot = self.bot.clone();
                let my_sign_checker = self.sign_checker.clone();
                let tx1 = tx.clone();
                let sign_handle = tokio::spawn(async {
                    handle_tx_thread(tx1, my_bot, my_sign_checker).await;
                });

                let my_lightning_checker = self.lightning_checker.clone();
                let tx2 = tx.clone();
                let my_bot2 = self.bot.clone();
                let lightning_handle = tokio::spawn(async {
                    handle_tx_lightning(tx2, my_bot2, my_lightning_checker).await;
                });

                sign_handle.await.unwrap();
                lightning_handle.await.unwrap();
            }
            Err(e) => {
                error!(
                    "Failed to deserialize transaction: received: {:?},{}",
                    tx_data, e
                );
            }
        }
    }
}

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

async fn handle_tx_thread(tx: Transaction, bot: Arc<TgBot>, checker: Arc<SignChecker>) {
    let txid = tx.compute_txid();
    let mut exist = false;
    let mut input_idx = 0;
    for (idx, input) in tx.input.iter().enumerate() {
        if input.witness.len() <= 0 || input.witness.len() > 4 {
            continue;
        }

        if input.witness[0].len() >= 32 {
            continue;
        }

        if checker.check_input_sign(&input) {
            continue;
        }

        // if input.witness.len() >= 1
        //     && lightning::is_swept_lightning_anchor(&hex::encode(&input.witness[1]))
        // {
        //     warn!(
        //         "Received transaction hash: {}. Swept Lightning Anchor",
        //         txid
        //     );
        //     continue;
        // }

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

async fn handle_tx_lightning(tx: Transaction, bot: Arc<TgBot>, checker: Arc<LightningChecker>) {
    let txid = tx.compute_txid();
    let input_idx = 0;
    let lightning_info = checker.check_input_sign(&tx);
    if lightning_info.is_none() {
        return;
    }

    let multisig = lightning_info.unwrap();

    info!(
        "Received transaction hash: {}, idx : {}, lightning channel closed",
        txid, input_idx
    );
    let msg = format!(
        "Lightning channel close, txid:{}, idx:{}, unlock_script_1: {}, unlock_script_2: {}",
        txid.to_string(),
        input_idx,
        hex::encode(multisig.unlock1),
        hex::encode(multisig.unlock2),
    );
    match bot.send_msg_to_topic(msg.as_str()).await {
        Ok(_) => {}
        Err(e) => error!("send msg to tg failed. {}", e),
    };
}
