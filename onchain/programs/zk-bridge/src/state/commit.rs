use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Commit {
    #[max_len(0)]
    pub data: Vec<u8>,
    pub bump: u8,
}
