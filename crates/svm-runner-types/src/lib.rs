use serde::{Deserialize, Serialize};
use solana_sdk::{
    account::{Account, AccountSharedData},
    hash::{hashv, Hash},
    pubkey::Pubkey,
    transaction::Transaction,
};

#[derive(Deserialize, Serialize, Debug)]
pub struct RampTx {
    pub is_onramp: bool,
    pub user: Pubkey,
    pub amount: u64,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ExecutionInput {
    pub accounts: RollupState,
    pub txs: Vec<Transaction>,
    pub ramp_txs: Vec<RampTx>,
}

pub type ExecutionOutput = Hash;

#[derive(Deserialize, Serialize, Debug)]
pub struct RollupState(pub Vec<(Pubkey, AccountSharedData)>);

// Temporary function used before adding the merklized state
pub fn hash_state(output: RollupState) -> Hash {
    let mut data = Vec::new();
    for (pk, account) in output.0.iter() {
        data.extend_from_slice(pk.as_ref());
        data.extend_from_slice(&bincode::serialize(account).unwrap());
    }
    hashv(&[data.as_slice()])
}

impl Into<onchain_types::RollupState> for RollupState {
    fn into(self) -> onchain_types::RollupState {
        let data = self
            .0
            .into_iter()
            .map(|(pk, account)| {
                let Account {
                    lamports,
                    data,
                    owner,
                    executable,
                    rent_epoch,
                } = account.into();
                (
                    onchain_types::Pubkey(pk.to_bytes()),
                    onchain_types::Account {
                        lamports,
                        data,
                        owner: onchain_types::Pubkey(owner.to_bytes()),
                        executable,
                        rent_epoch,
                    },
                )
            })
            .collect();
        onchain_types::RollupState(data)
    }
}

impl Into<onchain_types::RampTx> for RampTx {
    fn into(self) -> onchain_types::RampTx {
        onchain_types::RampTx {
            is_onramp: self.is_onramp,
            user: onchain_types::Pubkey(self.user.to_bytes()),
            amount: self.amount,
        }
    }
}

impl Into<onchain_types::ExecutionInput> for ExecutionInput {
    fn into(self) -> onchain_types::ExecutionInput {
        onchain_types::ExecutionInput {
            accounts: self.accounts.into(),
            txs: bincode::serialize(&self.txs).unwrap(),
            ramp_txs: self.ramp_txs.into_iter().map(|r| r.into()).collect(),
        }
    }
}
