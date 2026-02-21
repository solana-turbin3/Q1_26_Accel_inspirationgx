pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;
// mod tests;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("9jdh74PXnq1eZST56wMNtYMTrj1NM5Ns5AXEknXafhNz");

#[program]
pub mod gated_vault_transfer_hook {
    use super::*;

    pub fn create_vault(ctx: Context<VaultOperation>) -> Result<()> {
        ctx.accounts.create_vault(&ctx.bumps)
    }

    pub fn mint_token(ctx: Context<TokenFactory>, amount: u64, decimals: u8) -> Result<()> {
        ctx.accounts.mint_to_admin(amount, decimals)
    }

    pub fn add_to_whitelist(
        ctx: Context<WhitelistOperations>,
        address: Pubkey,
        _mint: Pubkey,
    ) -> Result<()> {
        ctx.accounts.add_to_whitelist(address, &ctx.bumps)
    }

    pub fn remove_from_whitelist(
        ctx: Context<WhitelistOperations>,
        address: Pubkey,
        _mint: Pubkey,
    ) -> Result<()> {
        ctx.accounts.remove_from_whitelist(address)
    }

    // deposit
    pub fn deposit<'info>(
        ctx: Context<'_, '_, '_, 'info, DepositWithdraw<'info>>,
        amount: u64,
    ) -> Result<()> {
        ctx.accounts.deposit(amount, &ctx.remaining_accounts)
    }
    // withdraw
    pub fn withdraw<'info>(
        ctx: Context<'_, '_, '_, 'info, DepositWithdraw<'info>>,
        amount: u64,
    ) -> Result<()> {
        ctx.accounts.withdraw(amount, &ctx.remaining_accounts)
    }
}
