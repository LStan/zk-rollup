use anchor_lang::prelude::*;
use onchain_types::CommittedValues;

use crate::constants::*;
use crate::errors::*;
use crate::state::*;

/// Derived as follows:
///
/// ```
/// let client = sp1_sdk::ProverClient::new();
/// let (pk, vk) = client.setup(YOUR_ELF_HERE);
/// let vkey_hash = vk.bytes32();
/// ```
const ZK_BRIDGE_VKEY_HASH: &str =
    "0x00e6c119f877ce29467d89e62b47f983177b85fddbd90ce988b6303b2f5d7f9b";
// "0x00508568011475c128053532cd7c60b9791f6edd5437e094c038a7445fba383d";

// #[derive(AnchorDeserialize, AnchorSerialize)]
// pub struct SP1Groth16Proof {
//     pub proof: Vec<u8>,
//     pub sp1_public_inputs: Vec<u8>,
// }

#[derive(Accounts)]
pub struct Prove<'info> {
    #[account(mut)]
    pub prover: Signer<'info>,
    #[account(
        seeds = [COMMIT_SEED_PREFIX, platform.id.as_ref(), prover.key().as_ref()],
        bump = commit.bump
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

impl Prove<'_> {
    pub fn handle(ctx: Context<Self>, proof: Vec<u8>) -> Result<()> {
        let vk = sp1_solana::GROTH16_VK_4_0_0_RC3_BYTES;
        sp1_solana::verify_proof(&proof, &ctx.accounts.commit.data, ZK_BRIDGE_VKEY_HASH, vk)
            .map_err(|_| PlatformError::InvalidProof)?;

        let committed_values: CommittedValues =
            bincode::deserialize(ctx.accounts.commit.data.as_slice()).unwrap();

        // msg!("commit data: {:?}", ctx.accounts.commit.data);
        // msg!("commit data len: {}", ctx.accounts.commit.data.len());

        // Check that ramps txs match the ones in the platform
        // Currently only check the count, could be improved to a hash of all txs
        if committed_values.input.ramp_txs.len() != ctx.accounts.platform.ramp_txs.len() {
            return Err(PlatformError::MissingRampTxs.into());
        }

        // Empty pending ramp txs
        ctx.accounts.platform.ramp_txs = vec![];

        // This can currently brick the platform, there should be a limit in number of ramp txs
        for ramp_tx in committed_values
            .input
            .ramp_txs
            .iter()
            .filter(|ramp_tx| !ramp_tx.is_onramp)
        {
            ctx.accounts.platform.withdraw += ramp_tx.amount;
        }

        // Update the platform state
        ctx.accounts.platform.last_state_hash = committed_values.output;

        Ok(())
    }
}
