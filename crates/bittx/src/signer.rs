use super::*;
use bitcoin::sighash::SighashCache;
use bitcoin::taproot::{ControlBlock, LeafVersion, TaprootBuilder, TaprootSpendInfo};
use bitcoin::{
    consensus::{encode::serialize_hex, serialize},
    sighash::Prevouts,
    PrivateKey,
};
use bitcoin::{TapLeafHash, TapSighash, TapSighashType};
use secp256k1::{Keypair, PublicKey, Secp256k1, SecretKey};

pub async fn sign_tx(
    tx: Transaction,
    prevouts: Vec<TxOut>,
    sign_idx: Vec<usize>,
) -> Result<Vec<u8>> {
    // 发送方的私钥
    let private_key = PrivateKey::from_wif("your_private_key_wif").unwrap();

    // 构建签名哈希缓存
    let mut sighash_cache = SighashCache::new(&tx);
    for idx in sign_idx.iter() {
        let sighash = sign_taproot_key_spend(sighash_cache, prevouts, *idx);
    }

    // let signature = private_key.sign(&sighash);
    // tx.input[0].script_sig = Script::new_p2pkh(&private_key.public_key(&network));

    // 将交易序列化为字节数组
    let raw_tx = serialize(&tx);
    info!("{}", serialize_hex(&tx));
    // // 广播交易
    // let txid = client.send_raw_transaction(&raw_tx).unwrap();
    // println!("Transaction broadcasted with txid: {}", txid);
    Ok(raw_tx)
}

fn sign_taproot_script_spend(
    private_key: PrivateKey,
    mut sighash_cache: SighashCache<&Transaction>,
    prevouts: Vec<TxOut>,
    idx: usize,
    script: ScriptBuf,
) -> TapSighash {
    let secp256k1 = Secp256k1::new();
    let sighash = sighash_cache
        .taproot_script_spend_signature_hash(
            idx,
            &Prevouts::All(&prevouts),
            TapLeafHash::from_script(&script, LeafVersion::TapScript),
            TapSighashType::All,
        )
        .unwrap();
    
    let keypair=Keypair::from_secret_key(&secp256k1, &private_key.inner);
    let sig = secp256k1.sign_schnorr(
        &secp256k1::Message::from_digest_slice(sighash.as_ref())
            .expect("should be cryptographically secure hash"),
        &keypair,
    );

    let witness = sighash_cache
        .witness_mut(commit_input)
        .expect("getting mutable witness reference should work");

    witness.push(
        Signature {
            sig,
            hash_ty: TapSighashType::Default,
        }
        .to_vec(),
    );

    sighash
}

fn sign_taproot_key_spend(
    mut sighash_cache: SighashCache<&Transaction>,
    prevouts: Vec<TxOut>,
    idx: usize,
) -> TapSighash {
    let sighash = sighash_cache
        .taproot_key_spend_signature_hash(idx, &Prevouts::All(&prevouts), TapSighashType::All)
        .unwrap();
    sighash
}
