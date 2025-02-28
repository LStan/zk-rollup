use crate::constants::*;
use crate::errors::*;
use crate::state::*;
use anchor_lang::prelude::*;
use anchor_lang::system_program::{transfer, Transfer};

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct AddRampTxArgs {
    pub is_onramp: bool,
    pub amount: u64,
}

#[derive(Accounts)]
#[instruction(args: AddRampTxArgs)]
pub struct AddRampTx<'info> {
    #[account(mut)]
    pub ramper: Signer<'info>,
    #[account(
        mut,
        realloc = 8 + Platform::INIT_SPACE + (platform.ramp_txs.len() + 1) * Ramp::INIT_SPACE,
        realloc::payer = ramper,
        realloc::zero = false,
        seeds = [
            PLATFORM_SEED_PREFIX,
            platform.id.as_ref(),
        ],
        bump = platform.bump
    )]
    pub platform: Account<'info, Platform>,
    #[account(
        init_if_needed,
        payer = ramper,
        space = 8 + Ramp::INIT_SPACE,
        seeds = [RAMP_SEED_PREFIX, platform.id.as_ref(), ramper.key().as_ref()],
        bump
    )]
    pub ramp: Account<'info, Ramp>,
    pub system_program: Program<'info, System>,
}

impl AddRampTx<'_> {
    pub fn handle(ctx: Context<Self>, args: AddRampTxArgs) -> Result<()> {
        if ctx.accounts.ramp.ramper.eq(&Pubkey::default()) {
            ctx.accounts.ramp.set_inner(Ramp {
                bump: ctx.bumps.ramp,
                ramper: ctx.accounts.ramper.key(),
                current_state_hash: ctx.accounts.platform.last_state_hash,
                pending_withdraw: 0,
            });
        }

        msg!("args.amount: {}", args.amount);

        if args.is_onramp {
            ctx.accounts.platform.deposit += args.amount;

            transfer(
                CpiContext::new(
                    ctx.accounts.system_program.to_account_info(),
                    Transfer {
                        from: ctx.accounts.ramper.to_account_info(),
                        to: ctx.accounts.platform.to_account_info(),
                    },
                ),
                args.amount,
            )?;
        } else {
            ctx.accounts.platform.withdraw += args.amount;
            if ctx.accounts.platform.withdraw > ctx.accounts.platform.deposit {
                return Err(PlatformError::InsufficientDeposits.into());
            }

            ctx.accounts.ramp.current_state_hash = ctx.accounts.platform.last_state_hash;
            ctx.accounts.ramp.pending_withdraw += args.amount;
        }

        ctx.accounts.platform.ramp_txs.push(RampTx {
            is_onramp: args.is_onramp,
            amount: args.amount,
            user: ctx.accounts.ramper.key(),
        });

        Ok(())
    }
}
