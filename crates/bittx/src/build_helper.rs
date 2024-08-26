use builder::{anchor, base};

use super::*;

pub async fn build_transer_tx(info: types::TransferInfo) -> Result<(Transaction, Vec<TxOut>)> {
    let utxos = utxo::gets_uspent_utxo(&info.sender).await?;

    // 创建交易对象
    let tx = base::build_transfer_tx(
        &info.sender,
        &info.recipient,
        info.amount,
        info.feerate,
        utxos,
    );
    Ok(tx)

    // 签名交易
    // let sighash = tx.signature_hash(
    //     0,
    //     &Script::new_p2pkh(&private_key.public_key(&network)),
    //     bitcoin::SigHashType::All.as_u32(),
    // );
    // let signature = private_key.sign(&sighash);
    // tx.input[0].script_sig = Script::new_p2pkh(&private_key.public_key(&network));

    // // 将交易序列化为字节数组
    // let raw_tx = serialize(&tx);

    // // 广播交易
    // let txid = client.send_raw_transaction(&raw_tx).unwrap();
    // println!("Transaction broadcasted with txid: {}", txid);
}

pub async fn build_anchor_tx(info: types::AnchorInfo) -> Result<(Transaction, Vec<TxOut>)> {
    let utxos = utxo::gets_uspent_utxo(&info.recipient).await?;
    if utxos.is_empty() {
        return Err(anyhow!("not found unspent utxo"));
    }

    let mut anchor_utxos = Vec::new();
    for (idx, out) in info.unlock_outs.iter().enumerate() {
        anchor_utxos.push(types::Utxo {
            out_point: OutPoint::from_str(&format!("{}:{}", info.anchor_txid, idx)).unwrap(),
            value: out.value,
            script_pubkey: out.script_pubkey.clone(),
        });
    }

    let my_utxo = utxos.get(0).unwrap();
    let (tx, prev_outs) =
        anchor::build_lightning_anchor_tx(my_utxo, anchor_utxos, info.unlock_bytes);

    Ok((tx, prev_outs))

    // 签名交易
    // let sighash = tx.signature_hash(
    //     0,
    //     &Script::new_p2pkh(&private_key.public_key(&network)),
    //     bitcoin::SigHashType::All.as_u32(),
    // );
    // let signature = private_key.sign(&sighash);
    // tx.input[0].script_sig = Script::new_p2pkh(&private_key.public_key(&network));

    // // 将交易序列化为字节数组
    // let raw_tx = serialize(&tx);

    // // 广播交易
    // let txid = client.send_raw_transaction(&raw_tx).unwrap();
    // println!("Transaction broadcasted with txid: {}", txid);
}

#[cfg(test)]
mod tests {
    use bitcoin::{
        consensus::encode::{deserialize_hex, serialize_hex},
        Transaction,
    };
    use datatypes::types;

    use super::{build_anchor_tx, lightning::check_lightning_channel_close};

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_build_anchor_tx() {
        let raw_tx = "0200000000010181dd8a52943508faea2249011bde2dcd04f79f315fdb57cf98eea132606221d90000000000f6108e80044a01000000000000220020a7edf64af17d189aa4fb72d0b470992598b9ccd4aca635de0ac457bba37435984a01000000000000220020ed1cf49beec11792218db3cb0260758ce9694c2dd3771233bec2154040b24cb03046000000000000220020375931514d22204f7b6bc404358ac59b380bd422f1e42a6474eb368057bb5b6e8a7e010000000000220020e4aa6f21574d8694eea0816ed7ecf9d907d105f5cea53a581d51f8a1aa1199370400473044022061e4234dbddcbe867296bec0a046235015782d8931ba30f4f341ea00f4c1d32d02205c5e3320d94a11fc48b6a40f4eafe6d2d971194a5ade591765671f37277d03a8014730440220522bc4661a9078e3bf70b8a46b1efb0404f05ff58a615220fbcc409e06560246022049ee2ef64811c17da6f063da4e92c37c3d4c85461bec19525df878a8ca7605ba014752210267d509df9e48bbe2b2f79efb266784b3cd2b643973ffa587a9ba67d30a5bb9ee21039bac7c4389aa48950d3511eefa7b892ec140013a339768571d4db094f2f26c0d52ae6c57fb20";
        let tx = deserialize_hex::<Transaction>(&raw_tx).unwrap();
        let unlock_info = check_lightning_channel_close(&tx).unwrap();
        let data = types::AnchorInfo {
            anchor_txid: tx.compute_txid().to_string(),
            unlock_bytes: vec![unlock_info.unlock1, unlock_info.unlock2],
            unlock_outs: vec![
                tx.output.get(0).unwrap().clone(),
                tx.output.get(1).unwrap().clone(),
            ],
            recipient: "bc1pdwy6qmwjhfng95v96avuer8za40vy7f66u5cphn9e09dzr6eemfstalyac".to_string(),
        };

        let res = build_anchor_tx(data).await;
        assert!(res.is_ok());

        println!("{}", serialize_hex(&res.unwrap().0))
    }
}
