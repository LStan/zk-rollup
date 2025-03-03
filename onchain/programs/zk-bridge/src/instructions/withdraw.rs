use anchor_lang::prelude::*;

use crate::constants::*;
use crate::errors::PlatformError;
use crate::state::platform::Platform;
use crate::state::ramp::Ramp;

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct WithdrawArgs {
    pub amount: u64,
}

#[derive(Accounts)]
#[instruction(args: WithdrawArgs)]
pub struct Withdraw<'info> {
    #[account(mut)]
    pub ramper: Signer<'info>,
    #[account(
        seeds = [
            PLATFORM_SEED_PREFIX,
            platform.id.as_ref(),
        ],
        bump = platform.bump
    )]
    pub platform: Account<'info, Platform>,
    #[account(
        mut,
        seeds = [
            RAMP_SEED_PREFIX,
            platform.id.as_ref(),
            ramper.key().as_ref(),
        ],
        bump = ramp.bump
    )]
    pub ramp: Account<'info, Ramp>,
}

impl Withdraw<'_> {
    pub fn validate(&self) -> Result<()> {
        if self.ramp.current_state_hash != self.platform.last_state_hash {
            return Err(PlatformError::InvalidStateHash.into());
        }

        Ok(())
    }

    pub fn handle(ctx: Context<Self>, args: WithdrawArgs) -> Result<()> {
        ctx.accounts.ramp.pending_withdraw -= args.amount;
        ctx.accounts.platform.sub_lamports(args.amount)?;
        ctx.accounts.ramper.add_lamports(args.amount)?;

        Ok(())
    }
}
