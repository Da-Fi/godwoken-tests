use crate::Spec;
use anyhow::{anyhow, Result};
use regex::Regex;
use std::env;
use std::{
    io::{BufRead, BufReader},
    process::{Command, Stdio},
};
//TODO: https://docs.rs/env_logger/0.8.3/env_logger/
//TODO: Redirect both stdout and stderr of child process to the same file

pub const CKB_SUDT_ACCOUNT_ID: u32 = 1;

pub struct CkbAsset;
impl Spec for CkbAsset {
    /// Case:
    ///   1. deposit CKB from layer1 to layer2
    ///   2. godwoken transfer from MINER to USER1
    ///   3. withdraw CKB from layer2 to layer1
    fn run(&self) {
        println!("\nCkbAsset Test Case: invoke account-cli to deposit -> transfer -> withdraw");

        let ckb_rpc: String =
            env::var("CKB_RPC").unwrap_or_else(|_| "http://127.0.0.1:8114".to_string());

        let mut miner = GodwokenUser {
            private_key: env::var("MINER_PRIVATE_KEY").unwrap_or_else(|_| {
                "0xdd50cac37ec6dd12539a968c1a2cbedda75bd8724f7bcad486548eaabb87fc8b".to_string()
            }),
            pub_ckb_addr: env::var("MINER_CKB_ADDR")
                .unwrap_or_else(|_| "ckt1qyqy84gfm9ljvqr69p0njfqullx5zy2hr9kq0pd3n5".to_string()),
            ckb_balance: 0,
            account_script_hash: None,
            gw_account_id: None,
        };
        let mut user1 = GodwokenUser {
            private_key: env::var("USER1_PRIVATE_KEY").unwrap_or_else(|_| {
                "0x6cd5e7be2f6504aa5ae7c0c04178d8f47b7cfc63b71d95d9e6282f5b090431bf".to_string()
            }),
            pub_ckb_addr: env::var("USER1_CKB_ADDR")
                .unwrap_or_else(|_| "ckt1qyqf22qfzaer95xm5d2m5km0f6k288x9warqnhsf4m".to_string()),
            ckb_balance: 0,
            account_script_hash: None,
            gw_account_id: None,
        };

        // call account-cli to deposit and get the script hash when the deposition finished.
        let script_hash_pattern = Regex::new(r"script hash: (0x.{64})").unwrap();
        let ckb_balance_pattern = Regex::new(r"ckb balance in godwoken is: (\d+)").unwrap();
        let account_id_pattern = Regex::new(r"Your account id: (\d+)").unwrap();

        let miner_deposit_stdout = account_cli()
            .arg("deposit")
            .args(&["--rpc", &ckb_rpc])
            .args(&["-p", &miner.private_key])
            .args(&["-c", "60000000000"]) // 600 CKBytes = 60,000,000,000 Shannons
            .stdout(Stdio::piped())
            .spawn()
            .unwrap()
            .stdout
            .unwrap();
        let mut last_ckb_balance_line = BufReader::new(miner_deposit_stdout)
            .lines()
            .filter_map(|line| line.ok())
            .filter_map(|line| {
                println!("{}", &line);
                // update account_script_hash
                if miner.account_script_hash.is_none() {
                    if let Some(cap) = script_hash_pattern.captures(&line) {
                        miner.account_script_hash = Some(cap.get(1).unwrap().as_str().to_string());
                        println!(
                            "=> update miner.account_script_hash to {:?}",
                            miner.account_script_hash
                        );
                    }
                }
                // update gw_account_id
                if miner.gw_account_id.is_none() {
                    if let Some(cap) = account_id_pattern.captures(&line) {
                        miner.gw_account_id = cap.get(1).unwrap().as_str().parse::<u32>().ok();
                        println!("=> update miner.gw_account_id to {:?}", miner.gw_account_id);
                    }
                }
                // filter the balance lines
                if line.starts_with("ckb balance") {
                    Some(line)
                } else {
                    None
                }
            })
            .last();
        if let Some(cap) =
            ckb_balance_pattern.captures(last_ckb_balance_line.unwrap().clone().as_str())
        {
            miner.ckb_balance = cap.get(1).unwrap().as_str().parse::<u128>().unwrap();
            // TODO: println
        };

        let user1_deposit_stdout = account_cli()
            .arg("deposit")
            .args(&["--rpc", &ckb_rpc])
            .args(&["-p", &user1.private_key])
            .args(&["-c", "30000000000"]) // 300 CKBytes = 30,000,000,000 Shannons
            .stdout(Stdio::piped())
            .spawn()
            .unwrap()
            .stdout
            .unwrap();
        last_ckb_balance_line = BufReader::new(user1_deposit_stdout)
            .lines()
            .filter_map(|line| line.ok())
            .filter_map(|line| {
                println!("{}", &line);
                // update account_script_hash
                if user1.account_script_hash.is_none() {
                    if let Some(cap) = script_hash_pattern.captures(&line) {
                        user1.account_script_hash = Some(cap.get(1).unwrap().as_str().to_string());
                        println!(
                            "=> update user1.account_script_hash to {:?}",
                            user1.account_script_hash
                        );
                    }
                }
                // update gw_account_id
                if user1.gw_account_id.is_none() {
                    if let Some(cap) = account_id_pattern.captures(&line) {
                        user1.gw_account_id = cap.get(1).unwrap().as_str().parse::<u32>().ok();
                        println!("=> update user1.gw_account_id to {:?}", user1.gw_account_id);
                    }
                }
                // filter the balance lines
                if line.starts_with("ckb balance") {
                    Some(line)
                } else {
                    None
                }
            })
            .last();
        if let Some(cap) =
            ckb_balance_pattern.captures(last_ckb_balance_line.unwrap().clone().as_str())
        {
            user1.ckb_balance = cap.get(1).unwrap().as_str().parse::<u128>().unwrap();
        };

        //TODO: if !exit_status.status.success()

        // let first_balance  = if let Some(cap) = balance_lines.next() {
        // 	u128::from_str_radix(cap.get(1).unwrap().as_str(), 10).unwrap()
        // } else { panic!("get first_balance error") };

        // let last_balance = if let Some(cap) = balance_lines.last() {
        // 	u128::from_str_radix(cap.get(1).unwrap().as_str(), 10).unwrap()
        // } else { panic!("get last_balance error") };
        // assert_eq!(last_balance - first_balance, 60000000000);
        // miner.ckb_balance = last_balance;

        let miner_balance_record = miner.get_balance().unwrap();
        println!("miner_balance_record: {}", miner_balance_record);
        assert_eq!(miner.ckb_balance, miner_balance_record);

        let user1_balance_record = user1.get_balance().unwrap();
        println!("user1_balance_record: {}", user1_balance_record);
        assert_eq!(user1.ckb_balance, user1_balance_record);

        // transfer
        println!("\nTransfer 10000000001 Shannons from miner to user1");
        let _transfer_status = account_cli()
            .arg("transfer")
            .args(&["--rpc", &ckb_rpc])
            .args(&["-p", &miner.private_key])
            .args(&["--amount", "10000000001"])
            .args(&["--fee", "100"])
            .args(&["--to-id", &user1.gw_account_id.unwrap().to_string()])
            .args(&["--sudt-id", &CKB_SUDT_ACCOUNT_ID.to_string()])
            .status()
            .expect("failed to transfer");
        // Note: balance is not updated now, waiting for confirmation.

        // withdraw
        println!("\nminer withdraw 40000000000 shannons (CKB) from godwoken");
        let mut _withdrawal_status = account_cli()
            .arg("withdraw")
            .args(&["--rpc", &ckb_rpc])
            .args(&["-p", &miner.private_key])
            .args(&[
                "--owner-ckb-address",
                "ckt1qyqy84gfm9ljvqr69p0njfqullx5zy2hr9kq0pd3n5",
            ]) // ckt1qyqy84gfm9ljvqr69p0njfqullx5zy2hr9kq0pd3n5
            .args(&["--capacity", "40000000000"]) // 40,000,000,000 Shannons = 400 CKBytes
            .status();

        // TODO: query balance after confirm and assert
        println!("miner_balance_record: {:?}", miner.get_balance());
        println!("user1_balance_record: {:?}", user1.get_balance());
        // FIXME:
        // thread '<unnamed>' panicked at 'assertion failed: `(left == right)`
        // left: `1529999987500`,
        // right: `1519999987499`', tests/src/specs/ckb_asset.rs:206:9
        // assert_eq!(miner.ckb_balance, miner_balance_record - 10000000001 - 40000000000);
        // assert_eq!(user1.ckb_balance, user1_balance_record + 10000000001);

        println!("\nuser1 withdraw 10000000000 shannons (CKB) from godwoken");
        _withdrawal_status = account_cli()
            .arg("withdraw")
            .args(&["--rpc", &ckb_rpc])
            .args(&["-p", &user1.private_key])
            .args(&["--owner-ckb-address", &user1.pub_ckb_addr])
            .args(&["--capacity", "10000000000"])
            .status();
    }
}

pub struct GodwokenUser {
    private_key: String,
    pub_ckb_addr: String,
    gw_account_id: Option<u32>, // FIXME: get account_id
    ckb_balance: u128,
    account_script_hash: Option<String>, //TODO: sudt_balance[]
}

impl GodwokenUser {
    fn get_balance(&mut self) -> Result<u128> {
        if self.gw_account_id.is_none() {
            // println!("missing gw_account_id");
            return Err(anyhow!("Missing gw_account_id: {:?}", self.gw_account_id));
        }
        // FIXME: get gw_account_id
        let pattern: Regex = Regex::new(r"[B|b]alance: (\d+)").unwrap();
        let balance_output = account_cli()
            .arg("get-balance")
            .args(&["--account-id", &self.gw_account_id.unwrap().to_string()])
            .output()
            .expect("failed to get-balance");
        let stdout_text = String::from_utf8(balance_output.stdout).unwrap_or("".to_string());
        let balance_str = if let Some(cap) = pattern.captures(&stdout_text) {
            if cap.len() > 1 {
                cap.get(1).unwrap().as_str()
            } else {
                "0"
            }
        } else {
            let err_text = String::from_utf8(balance_output.stderr).unwrap_or_default();
            return Err(anyhow!(
                "no balance logs returned: {} || {}",
                &err_text,
                &stdout_text
            ));
        };
        self.ckb_balance = u128::from_str_radix(balance_str, 10).unwrap();
        Ok(self.ckb_balance)
    }

    //TODO: fn deposit()
}

/// account_cli is built from godwoken-examples/packages/tools
fn account_cli() -> Command {
    let mut account_cli = if cfg!(target_os = "linux") {
        Command::new("./account-cli-linux")
    } else if cfg!(target_os = "macos") {
        Command::new("./account-cli-macos")
    } else {
        panic!("This OS is NOT supported yet.");
    };
    let godwoken_rpc: String =
        env::var("GODWOKEN_RPC").unwrap_or("http://127.0.0.1:8119".to_string());
    let lumos_config_file_path: String =
        env::var("LUMOS_CONFIG_FILE").unwrap_or("configs/lumos-config.json".to_string());
    account_cli
        .env("LUMOS_CONFIG_FILE", &lumos_config_file_path)
        .args(&["--godwoken-rpc", &godwoken_rpc]);
    account_cli
}
