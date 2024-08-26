use std::str::FromStr;

use super::*;
use bitcoin::{Address, Network};
use datatypes::types;

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
    block_height: Option<u32>,
    block_hash: Option<String>,
    block_time: Option<u64>,
}

pub async fn gets_uspent_utxo(addr: &str) -> Result<Vec<types::Utxo>> {
    gets_utxo(addr, true).await
}

async fn gets_utxo(addr: &str, confirmed: bool) -> Result<Vec<types::Utxo>> {
    let url = format!("{}/api/address/{}/utxo", MEMPOOL_URL, addr);
    debug!("{}", url);
    let address = Address::from_str(&addr)
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
