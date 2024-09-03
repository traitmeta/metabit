use std::str::FromStr;

use super::*;
use anyhow::Ok;
use bitcoin::{consensus::encode::serialize_hex, psbt::serialize, Address, Network, Transaction};
use datatypes::types;
use reqwest::Client;

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

pub async fn send_tx(tx: &Transaction) -> Result<String> {
    let url = format!("{}/tx", MEMPOOL_URL);
    let tx_hex = serialize_hex(tx);
    let client = Client::new();
    let response = client.post(url).body(tx_hex).send().await?;
    let resp = response.text().await?;
    Ok(resp)
}

pub async fn gets_uspent_utxo(addr: &str) -> Result<Vec<types::Utxo>> {
    gets_utxo(addr, true).await
}

async fn gets_utxo(addr: &str, confirmed: bool) -> Result<Vec<types::Utxo>> {
    let url = format!("{}/api/address/{}/utxo", MEMPOOL_URL, addr);
    debug!("{}", url);
    let address = Address::from_str(addr)
        .unwrap()
        .require_network(Network::Bitcoin)
        .unwrap();
    // 发起GET请求
    let response = reqwest::get(url).await?;

    debug!("{:?}", response);
    // 解析JSON响应
    let utxos: Vec<Utxo> = response.json().await?;

    // 输出解析后的数据
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

        let addr = "bc1qdx5yz3j59mgk6tfcedcn0ekud4exlg88s893j8";
        let utxos = gets_uspent_utxo(addr).await.unwrap();
        assert!(!utxos.is_empty());
        println!("{:?}", utxos);
    }
}
