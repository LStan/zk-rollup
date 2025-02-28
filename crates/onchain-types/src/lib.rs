use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Clone, Default)]
pub struct Pubkey(pub [u8; 32]);

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Clone, Default)]
pub struct Account {
    pub lamports: u64,
    pub data: Vec<u8>,
    pub owner: Pubkey,
    pub executable: bool,
    pub rent_epoch: u64,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct RampTx {
    pub is_onramp: bool,
    pub user: Pubkey,
    pub amount: u64,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ExecutionInput {
    pub accounts: RollupState,
    pub txs: Vec<u8>,
    pub ramp_txs: Vec<RampTx>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct RollupState(pub Vec<(Pubkey, Account)>);

pub type ExecutionOutput = [u8; 32];

#[derive(Deserialize, Serialize, Debug)]
pub struct CommittedValues {
    pub input: ExecutionInput,
    pub output: ExecutionOutput,
}
