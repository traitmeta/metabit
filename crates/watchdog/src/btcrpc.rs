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

    pub fn get_tx_out(&self, txid: &bitcoin::Txid, vout: u32) {
        match self.rpc.get_tx_out(txid, vout, None) {
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
}

#[cfg(test)]
mod tests {
    use super::BtcCli;
    use bitcoin::OutPoint;
    use std::str::FromStr;

    #[test]
    fn test_get_tx_out() {
        let btc_cli = BtcCli::new("http://127.0.0.1:8332", "", "");
        let outpoint = "903e78a5ce44c985459ff91fb9db49338b5901b8cfdbfa1aa875efc53eed4a2f:0";
        let out_point = OutPoint::from_str(outpoint).unwrap();

        btc_cli.get_tx_out(&out_point.txid, out_point.vout)
    }
}
