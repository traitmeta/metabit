use builder::{anchor, base, unsigned};

use super::*;

pub async fn build_transer_tx(
    info: types::TransferInfo,
    network: Option<Network>,
) -> Result<(Transaction, Vec<TxOut>)> {
    let utxos = utxo::gets_uspent_utxo(&info.sender).await?;

    let tx = base::build_transfer_tx(
        &info.sender,
        &info.recipient,
        info.amount,
        info.feerate,
        utxos,
        network,
    );
    Ok(tx)
}

pub async fn build_transer_tx_with_utxo(
    info: types::TransferInfo,
    utxos: Vec<types::Utxo>,
    network: Option<Network>,
) -> Result<(Transaction, Vec<TxOut>)> {
    let tx = base::build_transfer_tx(
        &info.sender,
        &info.recipient,
        info.amount,
        info.feerate,
        utxos,
        network,
    );
    Ok(tx)
}

pub async fn build_anchor_tx(
    info: types::AnchorInfo,
    my_utxo: types::Utxo,
) -> Result<(Transaction, Vec<TxOut>)> {
    let mut anchor_utxos = Vec::new();
    for (out, out_point) in info.unlock_outs.iter() {
        anchor_utxos.push(types::Utxo {
            out_point: *out_point,
            value: out.value,
            script_pubkey: out.script_pubkey.clone(),
        });
    }

    let (tx, prev_outs) =
        anchor::build_lightning_anchor_tx(&my_utxo, anchor_utxos, info.unlock_bytes);

    Ok((tx, prev_outs))
}

pub async fn build_batch_anchor_tx(
    info: types::AnchorsInfo,
    my_utxo: types::Utxo,
) -> Result<(Transaction, Vec<TxOut>)> {
    anchor::build_anchor_sweep_tx(&my_utxo, info.details)
}

pub async fn build_unsigned_tx(info: types::UnsignedInfo) -> Result<(Transaction, Vec<TxOut>)> {
    let utxos = utxo::gets_uspent_utxo(&info.recipient).await?;
    if utxos.is_empty() {
        return Err(anyhow!("not found unspent utxo"));
    }

    let mut unsigned_utxos = Vec::new();
    for (idx, input) in info.tx.input.into_iter().enumerate() {
        if idx as u32 == info.input_idx {
            unsigned_utxos.push(input);
        }
    }

    let my_utxo = utxos.first().unwrap();
    let (tx, prev_outs) = unsigned::build_unsigned_tx(my_utxo, info.input_out, unsigned_utxos);
    Ok((tx, prev_outs))
}

pub async fn build_unsigned_tx_with_receive_utxo(
    info: types::UnsignedInfo,
    utxo: types::Utxo,
) -> Result<(Transaction, Vec<TxOut>)> {
    let mut unsigned_utxos = Vec::new();
    for (idx, input) in info.tx.input.into_iter().enumerate() {
        if idx as u32 == info.input_idx {
            unsigned_utxos.push(input);
        }
    }

    let (tx, prev_outs) = unsigned::build_unsigned_tx(&utxo, info.input_out, unsigned_utxos);
    Ok((tx, prev_outs))
}

#[cfg(test)]
mod tests {
    use bitcoin::{
        consensus::encode::{deserialize_hex, serialize_hex},
        Amount, OutPoint, Transaction,
    };
    use datatypes::types;
    use mempool::utxo;

    use crate::build_helper::build_unsigned_tx;

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
                (
                    tx.output.get(0).unwrap().clone(),
                    OutPoint {
                        txid: tx.compute_txid(),
                        vout: 0,
                    },
                ),
                (
                    tx.output.get(1).unwrap().clone(),
                    OutPoint {
                        txid: tx.compute_txid(),
                        vout: 1,
                    },
                ),
            ],
            recipient: "bc1pdwy6qmwjhfng95v96avuer8za40vy7f66u5cphn9e09dzr6eemfstalyac".to_string(),
        };

        let utxos = utxo::gets_uspent_utxo(&data.recipient).await.unwrap();
        if utxos.is_empty() {
            eprintln!("not found unspent utxo");
            assert!(false);
        }
        let my_utxo = &utxos[0];
        let res = build_anchor_tx(data, my_utxo.clone()).await;
        assert!(res.is_ok());

        println!("{}", serialize_hex(&res.unwrap().0))
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_build_unsigned_tx() {
        let raw_tx = "02000000000102248378953c715b741364fea12dba6810873c9ce8b0ef82bd80e0e540d23c11710000000000ffffffff2e5d261b756cb3290f06b164dab8f33af27391bc9f979e868915d1cc037dffe60100000000ffffffff013c0400000000000016001492b8c3a56fac121ddcdffbc85b02fb9ef681038a02473044022044c75b0a4732f5f2b7d9364cdb329e75a9080faafc225cf70f69e9b5e0b3357602207c4e0cd82a6b479c3aab532757b1f359fde99990016b7b82ae0ef16bc44171860121030c7196376bc1df61b6da6ee711868fd30e370dd273332bfb02a2287d11e2e9c5030101fded0251690063036f7264010117746578742f68746d6c3b636861727365743d7574662d38004d08023c73637269707420646174612d733d2230783034656166386261613765303834343332326265303534616537313761366237633338373235623037656435633563363366616462626537643638393238633122207372633d222f636f6e74656e742f663830623933343636613238633565666337303366616230326265656262663465333265316263346630363361633237666564666437396164393832663263656930223e3c2f7363726970743e3c626f6479207374796c653d22646973706c61793a206e6f6e65223e3c2f626f64793e00000000000000004f505f42564d5f56321b5f02302f06dcdd2386badf6e1309a13554a5c60dfb9aea232499fdffc7bd7fea525ebef73d4f4a42d4e35092d4977f1a148df8cf2a92d0000820fdbbe145c7cdc0381d5585245e7632a3e866243151cde0518acf032ee2449ec820850c71211a1a00e1a9cc30e7feeaba41b911a1ef4d79c15ea9e4f9def14f1217cf4d612d67c1b0e746c0b4d9e925ffa4cc8d4d4c3880dc2d35634029215c28204a305b41ad9686aa208922caeae083353e60d051f328223a4995e1926966fbbc164a7a5ba1bd2653bd7d0d2592a593f23313a56bd2fe3badeca14293d37e14e6306ffe8545ff93b5f78206ed61a3f82f8afc0c6cf70baae29a6d348f6a3f1cbc8c25772fe3c25a5b640bb6faa1b10268643a10a99df3a2823254cc511e9408280435dc7a6b62f496484cbc46982248b84486566ba432b818fc546f6f43713c410c6c3a5dfe2d5c7b1414549bb46c3f66de779593f6be39e6e9ef2b99cdddb5c3f578ae1d974e88daa980878633f5318c43eeeaced3159c4682b5cea09411302c4664c67bcdb4923e8a0a5bc1b9900c1d518886454fb91301230615490c8661405ee1a9b32268813f3f3d40dcbf6232bc5252963bfad5f7b2303352afb6dc8d2399f5f8b74e7e834152f73fb928f0c2c1e4d570bdc3cee7853fe83602fe7bab6ab92af82a4802006821c056d0fcbf9a2a28cf293a2918f7cdbc128be0410508d8628a9f79b085f1eb5a2c00000000";
        let tx = deserialize_hex::<Transaction>(&raw_tx).unwrap();
        let out = tx.output.get(1).unwrap();
        let data = types::UnsignedInfo {
            recipient: "bc1pue6g3pvghm6vp2a0wqnlu9ls4l835mm3mr2kq0lcmw6z8p5p2jasxemufj".to_string(),
            tx: tx.clone(),
            input_idx: 1,
            input_out: out.clone().clone(),
        };

        let res = build_unsigned_tx(data).await;
        assert!(res.is_ok());

        println!("{}", serialize_hex(&res.unwrap().0))
    }
}
