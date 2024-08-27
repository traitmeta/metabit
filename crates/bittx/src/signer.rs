use super::*;
use bitcoin::sighash::SighashCache;
use bitcoin::taproot::LeafVersion;
use bitcoin::{
    consensus::encode::serialize_hex, sighash::Prevouts, taproot::Signature, PrivateKey,
};
use bitcoin::{TapLeafHash, TapSighash, TapSighashType};
use secp256k1::{Keypair, Secp256k1};

pub async fn sign_tx(
    wif: String,
    tx: Transaction,
    prevouts: Vec<TxOut>,
    sign_idx: Vec<usize>,
) -> Result<Transaction> {
    let private_key = PrivateKey::from_wif(wif.as_str()).unwrap();
    let mut tx = tx;
    for idx in sign_idx.iter() {
        sign_taproot_key_spend(private_key, &mut tx, &prevouts, *idx);
    }

    info!("{}", serialize_hex(&tx));
    Ok(tx)
}

pub fn sign_taproot(
    private_key: PrivateKey,
    mut tx: Transaction,
    prevouts: Vec<TxOut>,
    idx: usize,
    script: Option<ScriptBuf>,
) {
    match script {
        Some(s) => sign_taproot_script_spend(private_key, &mut tx, &prevouts, idx, s),
        None => sign_taproot_key_spend(private_key, &mut tx, &prevouts, idx),
    };
}

fn sign_taproot_script_spend(
    private_key: PrivateKey,
    tx: &mut Transaction,
    prevouts: &[TxOut],
    idx: usize,
    script: ScriptBuf,
) -> TapSighash {
    let mut sighash_cache = SighashCache::new(tx);
    let secp256k1 = Secp256k1::new();
    let sighash = sighash_cache
        .taproot_script_spend_signature_hash(
            idx,
            &Prevouts::All(prevouts),
            TapLeafHash::from_script(&script, LeafVersion::TapScript),
            TapSighashType::Default,
        )
        .unwrap();

    let keypair = Keypair::from_secret_key(&secp256k1, &private_key.inner);
    let sig = secp256k1.sign_schnorr(
        &secp256k1::Message::from_digest_slice(sighash.as_ref())
            .expect("should be cryptographically secure hash"),
        &keypair,
    );

    let witness = sighash_cache
        .witness_mut(idx)
        .expect("getting mutable witness reference should work");

    witness.push(
        Signature {
            signature: sig,
            sighash_type: TapSighashType::Default,
        }
        .to_vec(),
    );

    sighash
}

fn sign_taproot_key_spend(
    private_key: PrivateKey,
    tx: &mut Transaction,
    prevouts: &[TxOut],
    idx: usize,
) -> TapSighash {
    let mut sighash_cache = SighashCache::new(tx);
    let secp256k1 = Secp256k1::new();
    let sighash = sighash_cache
        .taproot_key_spend_signature_hash(idx, &Prevouts::All(prevouts), TapSighashType::Default)
        .unwrap();
    let keypair = Keypair::from_secret_key(&secp256k1, &private_key.inner);
    let sig = secp256k1.sign_schnorr(
        &secp256k1::Message::from_digest_slice(sighash.as_ref())
            .expect("should be cryptographically secure hash"),
        &keypair,
    );

    let witness = sighash_cache
        .witness_mut(idx)
        .expect("getting mutable witness reference should work");

    witness.push(
        Signature {
            signature: sig,
            sighash_type: TapSighashType::Default,
        }
        .to_vec(),
    );

    sighash
}
