use super::*;

pub fn check_witness_with_prev_txs(
    tx: &Transaction,
    prev_txs: Vec<Transaction>,
) -> Option<Vec<usize>> {
    let mut res = vec![];
    for (i, input) in tx.input.iter().enumerate() {
        println!("Input index: {}", i);
        let prev_tx = prev_txs.get(i).unwrap();
        let check_result = check_witness_signed(input, prev_tx);
        if !check_result {
            println!("    Witness is not valid for input index {}.", i);
            res.push(i);
        }
    }

    if res.len() == 0 {
        return None;
    }

    Some(res)
}

pub fn check_witness_signed(input: &TxIn, prev_tx: &Transaction) -> bool {
    if input.witness.len() <= 1 {
        return true;
    }

    let prev_out = prev_tx
        .output
        .get(input.previous_output.vout as usize)
        .unwrap();
    if !(prev_out.script_pubkey.is_p2wsh() || prev_out.script_pubkey.is_p2tr()) {
        return true;
    }

    // try parse witness to script
    let script_instructions = Script::from_bytes(&input.witness[1]).instructions();
    for instruction in script_instructions {
        match instruction {
            Ok(Instruction::Op(opcode)) => {
                println!("    OP Code: {:?}", opcode);
                if opcode == OP_CHECKMULTISIG
                    || opcode == OP_CHECKMULTISIGVERIFY
                    || opcode == OP_CHECKSIG
                    || opcode == OP_CHECKSIGVERIFY
                {
                    println!("    Witness is valid for P2WSH or P2TR.");
                    return true;
                }
            }
            Ok(Instruction::PushBytes(_)) => {
                continue;
            }
            Err(e) => {
                println!("    Error decoding script: {:?}", e);
            }
        }
    }

    false
}

pub fn check_input_signed(input: &TxIn, prev_out: &TxOut) -> bool {
    if input.witness.len() <= 1 {
        return true;
    }

    if !(prev_out.script_pubkey.is_p2wsh() || prev_out.script_pubkey.is_p2tr()) {
        return true;
    }

    // try parse witness to script
    let script_instructions = Script::from_bytes(&input.witness[1]).instructions();
    for instruction in script_instructions {
        match instruction {
            Ok(Instruction::Op(opcode)) => {
                println!("    OP Code: {:?}", opcode);
                if opcode == OP_CHECKMULTISIG
                    || opcode == OP_CHECKMULTISIGVERIFY
                    || opcode == OP_CHECKSIG
                    || opcode == OP_CHECKSIGVERIFY
                {
                    println!("    Witness is valid for P2WSH or P2TR.");
                    return true;
                }
            }
            Ok(Instruction::PushBytes(_)) => {
                continue;
            }
            Err(e) => {
                println!("    Error decoding script: {:?}", e);
            }
        }
    }

    false
}


pub fn check_witness(tx: &Transaction, prev_outs: Vec<TxOut>) {
    // parse witness data
    for (i, input) in tx.input.iter().enumerate() {
        println!("Input index: {}", i);
        if input.witness.len() <= 1 {
            continue;
        }

        let prev_out = prev_outs.get(i).unwrap();
        if !(prev_out.script_pubkey.is_p2wsh() || prev_out.script_pubkey.is_p2tr()) {
            continue;
        }

        // try parse witness to script
        let script_instructions = Script::from_bytes(&input.witness[1]).instructions();
        for instruction in script_instructions {
            match instruction {
                Ok(Instruction::Op(opcode)) => {
                    println!("    OP Code: {:?}", opcode);
                }
                Ok(Instruction::PushBytes(bytes)) => {
                    println!("    Data: {:?}", bytes);
                }
                Err(e) => {
                    println!("    Error decoding script: {:?}", e);
                }
            }
        }
    }
}

pub fn check_unsigned_input(tx: Transaction) -> Option<usize> {
    // parse witness data
    for (i, input) in tx.input.iter().enumerate() {
        if input.witness.len() <= 1 {
            continue;
        }

        let signed = is_signed_witness(&input.witness);
        if !signed {
            return Some(i);
        }
    }

    None
}

pub fn is_signed_witness(witness: &Witness) -> bool {
    if let Some(redeem_script_bytes) = witness.last() {
        let redeem_script = Script::from_bytes(redeem_script_bytes);
        redeem_script
            .instructions()
            .any(|instruction| match instruction {
                Ok(opcode) => {
                    opcode == Instruction::Op(OP_CHECKMULTISIG)
                        || opcode == Instruction::Op(OP_CHECKMULTISIGVERIFY)
                        || opcode == Instruction::Op(OP_CHECKSIG)
                        || opcode == Instruction::Op(OP_CHECKSIGVERIFY)
                }
                _ => false,
            })
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitcoin::blockdata::transaction::Transaction;
    use bitcoin::consensus::encode::deserialize_hex;

    #[test]
    fn tx_all_signed_test() {
        let raw_tx = "02000000000103507aa10a8824fda9e3a213d6d94a97b3dd6e7f993189c0316e516bc904344be3010000000010000000507aa10a8824fda9e3a213d6d94a97b3dd6e7f993189c0316e516bc904344be3000000000010000000625890453cd31111d578d3f0918ae7d8f1dc729d267b0ebe88bef4594c1ceca0010000000010000000010000000000000000036a010002002821031ed510a5fcd7ec5fe79ddc0d51914b3585f5c3ae444e5994581ecd0e06f187d8ac736460b2680200282103937af05b0c3493b5ca9380cc2e5ad52bbf02e19ecaabd9474b844616faf0642dac736460b268020028210380cf1f0ed09ba90ff2c80871512169ce72eaa1a9a1187136549c5600762290e5ac736460b26800000000";

        let tx = deserialize_hex::<Transaction>(&raw_tx).unwrap();
        let unsigned_input = check_unsigned_input(tx);
        assert!(unsigned_input.is_none());
    }

    #[test]
    fn tx_have_unsigned_test() {
        let raw_tx = "0200000000010285238518173326623fdae44c79edc3250f3e8607afbb1415cd74d8a7d2d39712010000000010000000e0e9053a4fc6c353293671887ca22687b0334720740628ce9611abb967dfb3340100000000ffffffff01c90100000000000016001492b8c3a56fac121ddcdffbc85b02fb9ef681038a0221028e5f26ab30ff467d7073374a9d646501fbdbc74b9f65e9029e0a848715fb7d870c093006020103020103017cac030101fd4c0451690063036f7264010117746578742f68746d6c3b636861727365743d7574662d38004d08023c73637269707420646174612d733d2230786364633039653933396366346432346437343261363430316530666365356635303261633530633961633236373937356339346630353430323161663262396622207372633d222f636f6e74656e742f663830623933343636613238633565666337303366616230326265656262663465333265316263346630363361633237666564666437396164393832663263656930223e3c2f7363726970743e3c626f6479207374796c653d22646973706c61793a206e6f6e65223e3c2f626f64793e000000000000000062766d76341b9f04c88ec338467c118dd8b93480a7b619451fdc5a0bdb4a0076785948b25b60ce52916ca796d07e15b15055c068d96ca793ed6614245c598881449451ee3dfc5d430c0c0239d91f6c8ad4340a0fb3883f8b1b806847bb4fe5c936c67703479a7a7e8240a0a11f03e148a751c082986f9d08acd5c5ccffa0fc1a24a9ddb65e3580c563abac5eb7b1d2ca2032558a828440d108c7e26cb601c993b029c52a7645241606270a12a780d901dbaca28622de7983db9b9a4b45d3a5c10f5c53e33372f3ef1f08d45a2d7accce73de526f173c4e0b99d4f55f7629d0891bc791b57c427d01ecb4f31113df1e1e94d0529a91416037b2fe74df80777b3deb2678b28d41e04d5542b219930609142c6449b1a046cb2ed8143489b544c09653c2e44807b6413b65a2f7d1a3654d0802f83a09374f5977df86698c9bfe096a226fe5ebbcfa0573ec0977dae708f71d20d7aa610699a967ec2c7793047d7398495ee08ea3ff55286c0e17e3a2f68650cc6fb90e3baf474fe7d0ebca8ac3e824054a9c890cd9e2b312ed12e80446c419a743ccc168c9123378618e05acb150287a4353699b049c2ed9a79fa393960c2dd3f9d826b6c70409cfa32cfa75ed7353694946bc2865ed41c8011f2993ce9ab00372c86e1a69305f6b56adf7bc2749d7a1e307ac782b4132f263289cb0b9e4595134a6f9f24dd734d62e08839528cec6a2e88cd9304607310030fb6c4208b978cd3161c2a09501afad33f92172c26c317301fecc15f3a58d04cd0c02abbf7eff128e4e61c3a04f0e32a6747569abcd382f647fb51429870c52074228efe1e963eca45d63190c807e4820bc2230aeb7ba898fbea22a6b692074300601fb8a4a06a308b22fe2357a40d4de73e0582c199badd3e208b88885e292513971489e1493f312c3007c3fc9591d0ea114f1b6654af47f3a1b748847a46618c91bb84a9bff61857ec1f36a60d843d6e57649dc9bb1f64f27eab16f6b7f4d49878cea72b9defebd7462fd2f13b475077e2ea98e04f701bca384268a294c9c821013834b208931ba4c42e03d581fd804133446cfda6451a0ad42f10207ad15a78731d4ef6b6d72a71ded7b8faf587ffc8f3f0d1fcaf2af05d6e0b5ad6b214a6f7e69fbcfefb968c5a4f72d12acf35f113ac0d2148bf96ff9d6b8f89148e5180d006821c0053d02842250ee6affd17a4ec4cbfa79a39d4259f9424db9576768fdc11de4e700000000";

        let tx = deserialize_hex::<Transaction>(&raw_tx).unwrap();
        let unsigned_input = check_unsigned_input(tx);
        assert!(unsigned_input.is_some());
        assert_eq!(unsigned_input.unwrap(), 1);
    }

    #[test]
    fn tx_have_unsigned_with_p2wpk_test() {
        let raw_tx = "02000000000102fae3a967247516d8775b0bcd5e746774bc0e4984f8b9e3a0f40384125b4724cd0000000000ffffffffb43799c9a61437309ba6dc0e3439b46a5900d67c97ebc203196a0d4cade66d560000000000ffffffff01e40600000000000016001492b8c3a56fac121ddcdffbc85b02fb9ef681038a024730440220418e31398367a2fec90941a70abe63cfe25f91105d29b9f0fc8e69afce7fa0eb02207d01d54551b8897df2e9e452864142689aaafd0e6e567b7ecc9c265380fbbdf20121030c7196376bc1df61b6da6ee711868fd30e370dd273332bfb02a2287d11e2e9c5030101fd030351690063036f7264010117746578742f68746d6c3b636861727365743d7574662d38004d08023c73637269707420646174612d733d2230786339643035366135373231633366613864343235633362333238663463386436326565383531613266326661623631343661643836333761323935373066633222207372633d222f636f6e74656e742f663830623933343636613238633565666337303366616230326265656262663465333265316263346630363361633237666564666437396164393832663263656930223e3c2f7363726970743e3c626f6479207374796c653d22646973706c61793a206e6f6e65223e3c2f626f64793e000000000000000062766d76341b560250af066c6318be694891e7297e9522b3b92855c7b4573147a8f4cbb5f7191ccb56755e7261d7b2ade7649302826290b5c89f35900354df03e6a0a0407f80907c1a18a7530598c05bd540a323a504c1069c60a001c9b6f382e78506be4e0ee87ec1a31cc40fc8510bc20c4a8085870c40807a0ce05b553b28a3defb4b5d81a7c3d3b891ce9e4f3f3fbf9baf867ea485ec283f8a3797fefd6a1064cd70af31c8d7d6a51b9adb7c1ea68c53e503c93511a0280389805632c58827b9648e7157c9a845540aed1910323aa436520a54496d95a0ced8d1ceae9a5b221ffafaedde47f7c08a66cc03fde8971288e37fadcf1ea47e2cb5c66566f2714f089df8f3ee897574d874c5808df5f7c588aa80a40d78162e7e5bf169c7cb22c75e700e0884232b61803c450e6a4cd2cb3993a4d2606c2cb1981a2c1557800431d2590e885582d6446b86d86867670d34d8c0e22f39ccd69b8eb61d863dfaacf1f0c667dd1c96f148dd869fe68ed4685b159d9706eaed6b1162d68f6bbab92d37981bff682ad826656b3fcbf7a9037cf904e3231863be4a63ac11422b6789d1463a44a05252ea194288550ab4641a61015c19091631006b9c109a39c389d4ea31e1919963df6921933d1789ea85798d3b5f89bb69ff8f29355efc2d18fa8cae31bd32da4e39fd2acfdb50523ff565a42f4c25ac14255dbcb98cd8a96ca878df06006821c1cb99fdef8d3033578d7097a37d4b9df2034b22deda94525a263e0c451ae0435300000000";
        let tx = deserialize_hex::<Transaction>(&raw_tx).unwrap();

        let prev_tx_raw_01 = "020000000001029d272d3644f4ace628eade757c075d4984b0ff73425fcacc3faca9e3443f7ae40000000000ffffffff335d226b78077c1cb772397cdf11ea8de1ce906242afab28c08b6b71da1d0d910100000000ffffffff01270600000000000016001492b8c3a56fac121ddcdffbc85b02fb9ef681038a024730440220323d0076caf5bda876eee325c5c60b3bb8281c0ddb401cf2418dbf7c79df509d02204d8293b1f4fb893ecd9ef82f5ab7129153418d3df40bdbd8742a784eaebf055f0121030c7196376bc1df61b6da6ee711868fd30e370dd273332bfb02a2287d11e2e9c5030101fdf50251690063036f7264010117746578742f68746d6c3b636861727365743d7574662d38004d08023c73637269707420646174612d733d2230783535666333626330373163613531633032396430366238386536333562633234646630383435396632636530646430646462386665646632623161613263623122207372633d222f636f6e74656e742f663830623933343636613238633565666337303366616230326265656262663465333265316263346630363361633237666564666437396164393832663263656930223e3c2f7363726970743e3c626f6479207374796c653d22646973706c61793a206e6f6e65223e3c2f626f64793e000000000000000062766d76341b5c0290ae066ca73351ec428a1a1de7be2ca4e9cdb2a37114a426cf4227fe7f85daccee1ed2c34dae1f150aa004473249090012d5026d8ed706c6e960373d142543c962fe9b9c1c10f8c5b6e70607944128394481f0662fb0c16030ec80c572a6b56c8779ffb9b60bb485ef9bc6226fa77b09733d639f09122e7d610367351374532b266123ebc3c5ff52a534bd6edc1cd449d32b7ed1e24ecea44a6704e1bddd1beb1c92b0c67b2527506b21b53411000d183452f140019d70d109cb49290a4a298c71434f6f7d90df8dee8398a61e37f82beaca9aeddcd5bad1bbfba83674c7bb7b817e2d90bddf0693f44cbbe0e1e75343c10163fb9cabe7db22c9f1b453e2ddaae5a7d9830707585324f8f68795ae9318093311e384721381cb09b400807e226ae5140181f68a4cc4494452dc5af45c0372728e81d41cb9b21b7a7aea15397dc2cc8fde3d0bdae7c31504c1acdd81da5bb7afdefda9518fa2928ee40ed6cbe5cb0fa76d74f5fcee3d76df007c1a68d0cae67c5cb733de901ee05b100fd60598bf9ab530d0e9858310394160c83d774e7b1096988996c5606102982565b9206951b910b5475241ba3644a334fffa880a90efe1a72a47f1655f96af1f507fbe5e5e973c3cebf447c943d9a213f5e0a79f0bfebf76d3d17506befa58fbd8425e7d22d41c2ff071c5c3071fd301006821c08864a1124013327b0fb7c49edd4b61f8b2de6fd5d37a840b99f73dc6fb23396400000000";
        let prev_tx_01 = deserialize_hex::<Transaction>(&prev_tx_raw_01).unwrap();

        let prev_tx_raw_02 = "02000000000101335d226b78077c1cb772397cdf11ea8de1ce906242afab28c08b6b71da1d0d910000000000fdffffff022c05000000000000225120f65e080b1450b51772a2345fd66a3e5e7913de5775294ac8969f33cf0c26b9715a5f01000000000022512075088bfac52b57aefff70f7ac9a6d2ae1e84b5bba7c4d9d7a130e57f8205a2b3014015424cf044180a6509213b8ea59aa915229b9bde83a268d4c3f7190a4641eb20e6b4fcf2dfa465a626ac818d8de50a5637db59ab8c7b56e91985f701c74fe6e100000000";
        let prev_tx_02 = deserialize_hex::<Transaction>(&prev_tx_raw_02).unwrap();

        let prev_txs = vec![prev_tx_01, prev_tx_02];
        let unsigned_input = check_witness_with_prev_txs(&tx, prev_txs);
        assert!(unsigned_input.is_some());
        assert_eq!(unsigned_input.unwrap().len(), 1);
    }
}
