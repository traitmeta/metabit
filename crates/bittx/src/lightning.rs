use builder::anchor::build_anchor_redeem_script;
use types::AnchorUnlockDetail;

use super::*;

// this is the simple implementation
pub fn check_lightning_channel_close(tx: &Transaction) -> Option<types::AnchorUnlockInfo> {
    if tx.input.len() != 1 {
        return None;
    }

    if tx.output.len() < 2 {
        return None;
    }

    // must be include two 330 output
    let mut count_of_330 = 0;
    for out in &tx.output {
        if out.value.to_sat() == 330 {
            count_of_330 += 1;
        }
    }

    if count_of_330 < 2 {
        return None;
    }

    let in_witness = &tx.input[0].witness;
    is_multisig_2_of_2(in_witness)
}

pub fn check_lightning_channel_closed(tx: &Transaction) -> Result<Vec<types::AnchorUnlockDetail>> {
    if tx.input.len() != 1 {
        return Err(anyhow!("tx input len must be 1"));
    }

    if tx.output.len() < 2 {
        return Err(anyhow!("tx output len must more then 2"));
    }

    let in_witness = &tx.input[0].witness;
    match is_multisig_2_of_2(in_witness) {
        Some(multi_sign) => {
            info!("found multi-sign of 2-2 : {:?}", multi_sign);
            let redeem_script1 = build_anchor_redeem_script(&multi_sign.unlock1);
            let redeem_script2 = build_anchor_redeem_script(&multi_sign.unlock1);
            let redeem_scripts = [redeem_script1, redeem_script2];
            let mut anchor_details = vec![];
            for (idx, out) in tx.output.iter().enumerate() {
                let mut redeem_script_hex = None;
                if out.script_pubkey == redeem_scripts[0].to_p2wsh() {
                    redeem_script_hex = Some(redeem_scripts[0].to_hex_string());
                } else if out.script_pubkey == redeem_scripts[1].to_p2wsh() {
                    redeem_script_hex = Some(redeem_scripts[1].to_hex_string());
                }

                if let Some(redeem_script_hex) = redeem_script_hex {
                    let detail = AnchorUnlockDetail {
                        redeem_script_hex,
                        out_value: out.value.to_sat(),
                        nsequence: 16,
                        vout: idx as u32,
                    };
                    anchor_details.push(detail);
                }
            }

            Ok(anchor_details)
        }
        None => Err(anyhow!("not 2-2 multisig")),
    }
}

// 2 <pubkey1> <pubkey2> 2 OP_CHECKMULTISIG
pub fn is_multisig_2_of_2(witness: &Witness) -> Option<types::AnchorUnlockInfo> {
    // witness[0] = empty（CHECKMULTISIG required bug fix value）
    // witness[1] and  witness[2] should be two public key
    // witness[3]  =  2 <pubkey1> <pubkey2> 2 OP_CHECKMULTISIG
    if witness.len() != 4 {
        return None;
    }

    // we think witness[3] = 2 <pubkey1> <pubkey2> 2 OP_CHECKMULTISIG
    let multisig_script = &witness[3];
    let script = Script::from_bytes(multisig_script);
    let mut iter = script.instructions();

    // must be start with OP_PUSHNUM_2
    if let Some(Ok(Instruction::Op(OP_PUSHNUM_2))) = iter.next() {
        // and then must be two public key
        let unlock1 = iter.next();
        let unlock2 = iter.next();

        if let (
            Some(Ok(Instruction::PushBytes(unlock1_bytes))),
            Some(Ok(Instruction::PushBytes(unlock2_bytes))),
        ) = (unlock1, unlock2)
        {
            // must end with OP_PUSHNUM_2 and OP_CHECKMULTISIG
            if let (
                Some(Ok(Instruction::Op(OP_PUSHNUM_2))),
                Some(Ok(Instruction::Op(OP_CHECKMULTISIG))),
            ) = (iter.next(), iter.next())
            {
                return Some(types::AnchorUnlockInfo {
                    unlock1: unlock1_bytes.as_bytes().to_vec(),
                    unlock2: unlock2_bytes.as_bytes().to_vec(),
                });
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitcoin::blockdata::transaction::Transaction;
    use bitcoin::consensus::encode::deserialize_hex;

    #[test]
    fn test_check_sniper_lightning() {
        let raw_tx = "0200000000010199737cff512e7207367804a536173cfbd11633feac0241283d3e8e8570f558ba0100000000b8e9b080044a010000000000002200202352053e1cd0b5f360d93bd39f324ac81ba82b9028252f2b02e9c468b9ba26f84a010000000000002200207535509faff2b5feb747ab8bb8eb12560c2f151a5ace5fe612526a8ca05f1febfc780200000000002200204ba3a03f6d2977476fa238320b2357d81f62ccba6caa104b456172af526612ca239b030000000000220020733a1726c25def1cb9b994c13f95cbe86d3cc48678edc88267bbba61426b173c040047304402207f3f9115b5484b8ebab72e4771ac8952575bd1ba466430dbe61e9b97429a4f2f022074fd8e526c9d94e5298f705db59b49ff9685ca998cca0687ff4a45db7aad6e4101483045022100cea8fabab14cea2a8d99ba3af21d8fc32d4504caeb7349b1b15a2ddd40febaf602200b197832477e13d669d049a54d4e579c942b4f3947bd01f59b11cece38a9dd9901475221024920e2293b862c6eeae69667af2654d0a31c36b0066a91d9b3a86994d3a910d62103a4a513fb72a6e352f0e42886cfaa7bbb433b690c687e791f718e4818c95210c552ae779bd520";

        let tx = deserialize_hex::<Transaction>(&raw_tx).unwrap();
        let result = check_lightning_channel_close(&tx);

        assert!(result.is_some());
        let unlock_info: types::AnchorUnlockInfo = result.unwrap();
        assert_eq!(
            hex::encode(unlock_info.unlock1),
            "024920e2293b862c6eeae69667af2654d0a31c36b0066a91d9b3a86994d3a910d6"
        );
        assert_eq!(
            hex::encode(unlock_info.unlock2),
            "03a4a513fb72a6e352f0e42886cfaa7bbb433b690c687e791f718e4818c95210c5"
        );
    }
}
