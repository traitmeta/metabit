use std::{str::FromStr, thread::sleep};

use super::*;
use bitcoin::{Address, Network};
use datatypes::types;
use tokio::time;
use tracing::{error, info};

#[derive(Debug, Deserialize)]
struct Utxo {
    txid: String,
    vout: u32,
    value: u64,
    status: Status,
}

#[derive(Debug, Deserialize)]
struct Status {
    confirmed: bool,
    _block_height: Option<u32>,
    _block_hash: Option<String>,
    _block_time: Option<u64>,
}

pub async fn gets_uspent_utxo(addr: &str) -> Result<Vec<types::Utxo>> {
    gets_utxo(addr, true).await
}

async fn gets_utxo(addr: &str, confirmed: bool) -> Result<Vec<types::Utxo>> {
    let url = format!("{}/api/address/{}/utxo", MEMPOOL_URL, addr);
    info!("{}", url);
    let address = Address::from_str(addr)
        .unwrap()
        .require_network(Network::Bitcoin)
        .unwrap();

    let mut utxos: Vec<Utxo> = vec![];
    for _i in 0..3 {
        match reqwest::get(url.clone()).await {
            Ok(response) => {
                debug!("{:?}", response);
                utxos = response.json().await?;
                break;
            }
            Err(e) => {
                error!("Error fetching utxo: {}", e);
                sleep(time::Duration::from_secs(10));
            }
        }
    }

    if utxos.is_empty() {
        return Err(anyhow!("not found utxo"));
    }

    let mut my_utxos: Vec<types::Utxo> = Vec::new();
    for utxo in utxos {
        let my_utxo = types::Utxo {
            out_point: OutPoint::from_str(&format!("{}:{}", utxo.txid, utxo.vout)).unwrap(),
            value: Amount::from_sat(utxo.value),
            script_pubkey: address.script_pubkey().clone(),
        };
        if confirmed {
            if utxo.status.confirmed {
                my_utxos.push(my_utxo);
            }
        } else {
            my_utxos.push(my_utxo);
        }
    }

    Ok(my_utxos)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_get_uspent_utxo() {
        tracing_subscriber::fmt()
            .with_test_writer() // 将日志输出到测试控制台
            .init();

        let addr = "bc1pue6g3pvghm6vp2a0wqnlu9ls4l835mm3mr2kq0lcmw6z8p5p2jasxemufj";
        let utxos = gets_uspent_utxo(addr).await.unwrap();
        println!("{:?}", utxos);
        assert!(!utxos.is_empty());
    }
}
