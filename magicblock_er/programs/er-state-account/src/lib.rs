#![allow(unexpected_cfgs)]
#![allow(deprecated)]

use anchor_lang::prelude::*;
use ephemeral_rollups_sdk::anchor::ephemeral;

mod instructions;
mod state;

use instructions::*;

declare_id!("CMPECrgdFbSGwSK8gSCD3wehkcNUbesyK8FYfS5Fsfjg");

#[ephemeral]
#[program]
pub mod er_state_account {

    use super::*;

    pub fn initialize(ctx: Context<InitUser>) -> Result<()> {
        ctx.accounts.initialize(&ctx.bumps)?;

        Ok(())
    }

    pub fn update(ctx: Context<UpdateUser>, new_data: u64) -> Result<()> {
        ctx.accounts.update(new_data)?;

        Ok(())
    }

    pub fn update_commit(ctx: Context<UpdateCommit>, new_data: u64) -> Result<()> {
        ctx.accounts.update_commit(new_data)?;

        Ok(())
    }

    pub fn request_randomness(ctx: Context<VrfUpdateCommit>, seed: u64) -> Result<()> {
        ctx.accounts.request_randomness(seed)
    }

    pub fn request_randomness_undelegated(ctx: Context<VrfUpdate>, seed: u64) -> Result<()> {
        ctx.accounts.request_randomness_undelegated(seed)
    }

    pub fn delegate(ctx: Context<Delegate>) -> Result<()> {
        ctx.accounts.delegate()?;

        Ok(())
    }

    pub fn undelegate(ctx: Context<Undelegate>) -> Result<()> {
        ctx.accounts.undelegate()?;

        Ok(())
    }

    pub fn close(ctx: Context<CloseUser>) -> Result<()> {
        ctx.accounts.close()?;

        Ok(())
    }

    pub fn delegated_update_commit(
        ctx: Context<CallBackDelegated>,
        randomness: [u8; 32],
    ) -> Result<()> {
        ctx.accounts.delegated_update_commit(randomness)
    }

    pub fn undelegated_update(
        ctx: Context<CallBackUnDelegated>,
        randomness: [u8; 32],
    ) -> Result<()> {
        ctx.accounts.undelegated_update(randomness)
    }
}
