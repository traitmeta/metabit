
use bitcoin::consensus::encode::serialize;
use bitcoin::network::constants::Network;
use bitcoin::util::bip32::ExtendedPrivKey;
use bitcoin::util::key::PrivateKey;
use bitcoin::blockdata::transaction::{Transaction, TxIn, TxOut};
use bitcoin::blockdata::script::Script;
use bitcoin::blockdata::opcodes;
use bitcoincore_rpc::{Auth, Client, RpcApi};

pub fn build_tx() {
    // 初始化比特币网络和RPC客户端
    let network = Network::Testnet;
    let rpc_url = "http://localhost:18332"; // 比特币节点的RPC URL
    let rpc_auth = Auth::UserPass("username".to_string(), "password".to_string());
    let client = Client::new(rpc_url.to_string(), rpc_auth).unwrap();

    // UTXO信息：通常通过RPC获取
    let utxo_txid = "your_utxo_txid".parse().unwrap();
    let utxo_vout = 0;
    let utxo_amount = Amount::from_sat(10000);

    // 发送方的私钥
    let private_key = PrivateKey::from_wif("your_private_key_wif").unwrap();

    // 接收方地址和金额
    let recipient_address = Address::from_str("recipient_address").unwrap();
    let recipient_amount = Amount::from_sat(9000);

    // 创建交易输入
    let txin = TxIn {
        previous_output: bitcoin::OutPoint {
            txid: utxo_txid,
            vout: utxo_vout,
        },
        script_sig: Script::new(),
        sequence: 0xffffffff,
        witness: Vec::new(),
    };

    // 创建交易输出
    let txout = TxOut {
        value: recipient_amount.as_sat(),
        script_pubkey: recipient_address.script_pubkey(),
    };

    // 创建交易对象
    let mut tx = Transaction {
        version: 2,
        lock_time: 0,
        input: vec![txin],
        output: vec![txout],
    };

    // 签名交易
    let sighash = tx.signature_hash(0, &Script::new_p2pkh(&private_key.public_key(&network)), bitcoin::SigHashType::All.as_u32());
    let signature = private_key.sign(&sighash);
    tx.input[0].script_sig = Script::new_p2pkh(&private_key.public_key(&network));

    // 将交易序列化为字节数组
    let raw_tx = serialize(&tx);

    // 广播交易
    let txid = client.send_raw_transaction(&raw_tx).unwrap();
    println!("Transaction broadcasted with txid: {}", txid);
}