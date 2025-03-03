pub mod constants;
pub mod errors;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("Bq5sTpeHWMCety13tmZqgYvDJoAAC4mAwBn33mZmuc41");

#[program]
pub mod zk_bridge {
    use super::*;

    pub fn create_platform(ctx: Context<CreatePlatform>, args: CreatePlatformArgs) -> Result<()> {
        CreatePlatform::handle(ctx, args)
    }

    /// Add a ramp transaction to the platform.
    ///
    /// **This can currently be used to DoS the platform by adding transactions faster than the sequencer can generate proofs.**
    pub fn add_ramp_tx(ctx: Context<AddRampTx>, args: AddRampTxArgs) -> Result<()> {
        AddRampTx::handle(ctx, args)
    }

    pub fn upload_commit(ctx: Context<UploadCommit>, args: UploadCommitArgs) -> Result<()> {
        UploadCommit::handle(ctx, args)
    }

    pub fn prove(ctx: Context<Prove>, proof: Vec<u8>) -> Result<()> {
        Prove::handle(ctx, proof)
    }

    #[access_control(ctx.accounts.validate())]
    pub fn withdraw(ctx: Context<Withdraw>, args: WithdrawArgs) -> Result<()> {
        Withdraw::handle(ctx, args)
    }
}
