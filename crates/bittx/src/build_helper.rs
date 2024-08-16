use super::*;

pub async fn build_transer_tx(info: types::TransferInfo) -> Result<(Transaction, Vec<TxOut>)> {
    let utxos = utxo::gets_uspent_utxo(&info.sender).await?;

    // 创建交易对象
    let tx = builder::build_transfer_tx(
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
