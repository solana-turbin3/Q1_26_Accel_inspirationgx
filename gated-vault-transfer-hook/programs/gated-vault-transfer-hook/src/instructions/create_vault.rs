use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{spl_pod::option::Nullable, Mint, TokenAccount, TokenInterface},
};

use crate::VAULT_SEED;
use crate::{error::VaultError, Vault};

#[derive(Accounts)]
pub struct VaultOperation<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        init,
        payer = owner,
        mint::decimals = 9,
        mint::authority = owner,
        extensions::transfer_hook::authority = owner,
        extensions::transfer_hook::program_id = hook_program_id.key(), // to add transfer hook program id
    )]
    pub mint: InterfaceAccount<'info, Mint>,

    /// CHECK: Program id of the tf hook
    pub hook_program_id: UncheckedAccount<'info>,

    #[account(
        init,
        payer = owner,
        seeds = [mint.key().as_ref(),VAULT_SEED],
        space = Vault::DISCRIMINATOR.len() + Vault::INIT_SPACE,
        bump
    )]
    pub vault_state: Account<'info, Vault>,

    #[account(
        init,
        payer = owner,
        associated_token::mint = mint,
        associated_token::authority = vault_state,
        associated_token::token_program = token_program
    )]
    pub vault_ata: InterfaceAccount<'info, TokenAccount>,

    pub system_program: Program<'info, System>,
    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

impl<'info> VaultOperation<'info> {
    pub fn create_vault(&mut self, bumps: &VaultOperationBumps) -> Result<()> {
        // check that vault does not already exist
        require!(
            Pubkey::is_none(&self.vault_state.owner),
            VaultError::VaultAlreadyExist
        );
        let mint = self.mint.key();
        let owner = self.owner.key();
        let vault_bump = bumps.vault_state;

        let vault = &mut self.vault_state;
        vault.set_inner(Vault {
            mint,
            owner,
            vault_bump,
        });

        Ok(())
    }

    // pub fn deposit(&mut self, amount: u64)
}

//   #[account(
//         mut,
//         associated_token::authority =
//     )]
//     pub owner_ata: InterfaceAccount<'info, TokenAccount>,
