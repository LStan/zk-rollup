use anchor_lang::prelude::*;

use crate::constants::*;
use crate::state::*;

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct UploadCommitArgs {
    pub commit_size: u64,
    pub offset: u64,
    pub commit_data: Vec<u8>,
}

#[derive(Accounts)]
#[instruction(args: UploadCommitArgs)]
pub struct UploadCommit<'info> {
    #[account(mut)]
    pub prover: Signer<'info>,
    #[account(
        init_if_needed,
        payer = prover,
        space = 8 + Commit::INIT_SPACE + args.commit_size as usize,
        seeds = [COMMIT_SEED_PREFIX, platform.id.as_ref(), prover.key().as_ref()],
        bump
    )]
    pub commit: Account<'info, Commit>,
    #[account(
        mut,
        seeds = [
            PLATFORM_SEED_PREFIX,
            platform.id.as_ref(),
        ],
        bump = platform.bump
    )]
    pub platform: Account<'info, Platform>,
    pub system_program: Program<'info, System>,
}

impl UploadCommit<'_> {
    pub fn handle(ctx: Context<Self>, args: UploadCommitArgs) -> Result<()> {
        let commit = &mut ctx.accounts.commit;
        if commit.bump != ctx.bumps.commit {
            commit.bump = ctx.bumps.commit;
            commit.data = vec![0; args.commit_size as usize];
        }
        msg!("commit size: {}", commit.data.len());
        msg!("commit data len: {}", args.commit_data.len());

        let offset = args.offset as usize;
        let end = (offset + args.commit_data.len()).min(commit.data.len());
        msg!("offset: {}, end: {}", offset, end);
        commit.data[offset..end].copy_from_slice(&args.commit_data);

        Ok(())
    }
}
