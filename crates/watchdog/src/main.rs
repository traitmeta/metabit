use bitcoin::{consensus::encode::deserialize, Transaction};
use tgbot::TgBot;
use watchdog::config;
use zmq::Context;

#[tokio::main]
async fn main() {
    let cfg = config::read_config();
    let context = Context::new();
    let subscriber = context.socket(zmq::SUB).unwrap();
    subscriber
        .connect(format!("tcp://{}:{}", cfg.bitcoin.zmq, cfg.bitcoin.zmq_port).as_str())
        .unwrap();
    subscriber.set_subscribe(b"rawtx").unwrap();
    println!("Subscribed to raw transactions...");
    let bot = TgBot::new(&cfg.tgbot.token, cfg.tgbot.chat_id, cfg.tgbot.tx_topic_id);

    loop {
        let _topic = subscriber.recv_msg(0).unwrap();
        let tx_data = subscriber.recv_bytes(0).unwrap();

        match deserialize::<Transaction>(&tx_data) {
            Ok(tx) => {
                for input in &tx.input {
                    if input.witness.len() > 0 && input.witness[0].len() < 64 {
                        let mut add = "";
                        if input.witness.len() > 3 {
                            add = "May be multisign";
                        }

                        println!("Received transaction hash: {}", tx.txid());
                        let msg = format!("TxID : {}, {}", tx.txid().to_string().as_str(), add);
                        bot.send_msg_to_topic(msg.as_str()).await;
                    }
                }
            }
            Err(_) => {
                // eprintln!("Failed to deserialize transaction: {}", e);
            }
        }
    }
}
