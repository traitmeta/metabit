use repo::Dao;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::broadcast::Sender;
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
    dao: Arc<repo::Dao>,
}

impl TxReceiver {
    pub async fn new(cfg: &config::Config) -> Self {
        let bot = TgBot::new(&cfg.tgbot.token, cfg.tgbot.chat_id, cfg.tgbot.tx_topic_id);
        let btccli = BtcCli::new(&cfg.bitcoin.endpoint, &cfg.bitcoin.user, &cfg.bitcoin.pass);
        let sign_checker = SignChecker::new(btccli);
        let lightning_checker = LightningChecker::new();
        let conn_pool = repo::conn_pool(&cfg.database).await.unwrap();
        let dao = Dao::new(conn_pool);
        Self {
            bot: Arc::new(bot),
            sign_checker: Arc::new(sign_checker),
            lightning_checker: Arc::new(lightning_checker),
            dao: Arc::new(dao),
        }
    }

    #[tracing::instrument(skip_all)]
    pub async fn handle_recv(
        &self,
        tx_data: Vec<u8>,
        sender: Sender<(Transaction, u32)>,
    ) -> Result<()> {
        if tx_data.is_empty() {
            return Ok(());
        }

        debug!("received from zmq : {:?}", tx_data);
        match deserialize::<Transaction>(&tx_data) {
            Ok(tx) => {
                debug!("received tx : {}", tx.compute_txid());
                let my_bot = self.bot.clone();
                let my_sign_checker = self.sign_checker.clone();
                let tx1 = tx.clone();
                let sender1 = sender.clone();
                let sign_handle = tokio::spawn(async {
                    handle_tx_thread(tx1, my_bot, my_sign_checker, sender1).await;
                });

                let my_lightning_checker = self.lightning_checker.clone();
                let tx2 = tx.clone();
                let my_bot2 = self.bot.clone();
                let dao = self.dao.clone();
                let lightning_handle = tokio::spawn(async {
                    handle_tx_lightning(tx2, my_bot2, my_lightning_checker, dao).await;
                });

                sign_handle.await?;
                lightning_handle.await?;
            }
            Err(e) => {
                error!(
                    "Failed to deserialize transaction: received: {:?},{}",
                    tx_data, e
                );
            }
        }

        Ok(())
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
        if input.witness.is_empty() {
            continue;
        }

        if input.witness[0].len() >= 32 {
            continue;
        }

        if checker.check_input_sign(input) {
            continue;
        }

        if !input.witness.is_empty()
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
        let msg = format!("txid:{},idx:{}", txid, input_idx);
        match bot.send_msg_to_topic(msg.as_str()).await {
            Ok(_) => {}
            Err(e) => error!("send msg to tg failed. {}", e),
        };
    }
}

async fn handle_tx_thread(
    tx: Transaction,
    bot: Arc<TgBot>,
    checker: Arc<SignChecker>,
    sender: Sender<(Transaction, u32)>,
) {
    if tx.is_coinbase() {
        return;
    }

    let txid = tx.compute_txid();
    let mut exist = false;
    let mut input_idx = 0;
    for (idx, input) in tx.input.iter().enumerate() {
        if input.witness.is_empty() || input.witness.len() > 4 {
            continue;
        }

        if checker.check_input_sign(input) {
            continue;
        }

        exist = true;
        input_idx = idx;
        break;
    }

    if exist {
        match sender.send((tx, input_idx as u32)) {
            Ok(_) => {}
            Err(e) => error!("send msg to channel failed. {}", e),
        }
        info!("Received transaction hash: {}, idx : {}", txid, input_idx);
        let msg = format!("txid:{},idx:{}", txid, input_idx);
        match bot.send_msg_to_topic(msg.as_str()).await {
            Ok(_) => {}
            Err(e) => error!("send msg to tg failed. {}", e),
        };
    }
}

async fn handle_tx_lightning(
    tx: Transaction,
    bot: Arc<TgBot>,
    checker: Arc<LightningChecker>,
    dao: Arc<Dao>,
) {
    let txid = tx.compute_txid();
    let input_idx = 0;
    let lightning_info = checker.check_anchor_closed(&tx);
    if lightning_info.is_none() {
        return;
    }
    let infos = lightning_info.unwrap();

    for info in infos {
        match dao.insert_anchor_tx_out(info).await {
            Ok(_) => {}
            Err(e) => error!("Error Insert anchor tx out: {:?}", e),
        }
    }

    info!(
        "Received transaction hash: {}, idx : {}, lightning channel closed",
        txid, input_idx
    );
    let msg = format!("Lightning channel close, txid:{}, idx:{}", txid, input_idx,);
    match bot.send_msg_to_topic(msg.as_str()).await {
        Ok(_) => {}
        Err(e) => error!("send msg to tg failed. {}", e),
    };
}
