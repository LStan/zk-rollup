use anchor_lang::prelude::*;

/// A platform is the account storing state waiting to be sent to the rollup
#[account]
#[derive(InitSpace)]
pub struct Platform {
    pub sequencer: Pubkey,
    pub id: Pubkey,
    pub last_state_hash: [u8; 32],
    #[max_len(30)]
    pub ramp_txs: Vec<RampTx>,
    pub deposit: u64,
    pub withdraw: u64,
    pub bump: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, InitSpace)]
pub struct RampTx {
    pub is_onramp: bool,
    pub user: Pubkey,
    pub amount: u64,
}
