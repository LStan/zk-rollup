use anchor_lang::prelude::*;

/// A platform is the account storing state waiting to be sent to the rollup
#[account]
#[derive(InitSpace)]
pub struct Ramp {
    pub ramper: Pubkey,
    pub current_state_hash: [u8; 32],
    pub pending_withdraw: u64,
    pub bump: u8,
}
