use super::*;
use bitcoin::TxOut;
use bitcoincore_rpc::{Auth, Client, RpcApi};

pub struct BtcCli {
    rpc: Client,
}

impl BtcCli {
    pub fn new(url: &str, user: &str, pass: &str) -> Self {
        let rpc: Client =
            Client::new(url, Auth::UserPass(user.to_string(), pass.to_string())).unwrap();
        Self { rpc }
    }

    pub fn get_unsepnt_tx_out(&self, txid: &bitcoin::Txid, vout: u32) {
        match self.rpc.get_tx_out(txid, vout, Some(true)) {
            Ok(Some(txout)) => {
                println!("TxOut found: {:?}", txout);
            }
            Ok(None) => {
                println!("No TxOut found for the given outpoint.");
            }
            Err(e) => {
                eprintln!("Error fetching TxOut: {:?}", e);
            }
        }
    }

    pub fn get_tx_out(&self, txid: &bitcoin::Txid, vout: u32) -> Result<TxOut> {
        match self.rpc.get_raw_transaction(&txid, None) {
            Ok(raw_tx) => {
                let tx: bitcoin::Transaction = raw_tx;
                let tx_out = &tx.output[vout as usize];

                println!("TxOut value: {}", tx_out.value);
                println!("TxOut script_pubkey: {}", tx_out.script_pubkey);
                Ok(tx_out.clone())
            }
            Err(e) => {
                eprintln!("Error fetching raw transaction: {:?}", e);
                Err(anyhow!("Error fetching raw transaction"))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::BtcCli;
    use crate::config;
    use bitcoin::OutPoint;
    use std::str::FromStr;

    #[test]
    fn test_get_tx_out() {
        let cfg = config::load_config("./config.toml");
        let btc_cli = BtcCli::new(&cfg.bitcoin.endpoint, &cfg.bitcoin.user, &cfg.bitcoin.pass);
        let outpoint = "903e78a5ce44c985459ff91fb9db49338b5901b8cfdbfa1aa875efc53eed4a2f:0";
        let out_point = OutPoint::from_str(outpoint).unwrap();

        let res = btc_cli.get_tx_out(&out_point.txid, out_point.vout);
        assert!(res.is_ok());
    }
}
