use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_2022::{mint_to_checked, MintToChecked},
    token_interface::{Mint, TokenAccount, TokenInterface},
};

use crate::{error::VaultError, state::Whitelist};

#[derive(Accounts)]
pub struct TokenFactory<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(
        mut,
        mint::decimals = 9,
        mint::authority = user,
        extensions::transfer_hook::authority = user,
        extensions::transfer_hook::program_id = hook_program_id.key(),
    )]
    pub mint: InterfaceAccount<'info, Mint>,

    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = mint,
        associated_token::authority = user,
        associated_token::token_program = token_program,
    )]
    pub source_token_account: InterfaceAccount<'info, TokenAccount>,
    /// CHECK: ExtraAccountMetaList Account, will be checked by the transfer hook
    #[account(mut)]
    pub extra_account_meta_list: UncheckedAccount<'info>,
    #[account(
        seeds = [b"whitelist"], 
        bump
    )]
    pub blocklist: Account<'info, Whitelist>,
    /// CHECK: Program id of the tf hook
    pub hook_program_id: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

impl<'info> TokenFactory<'info> {
    pub fn mint_to_admin(&mut self, amount: u64, decimals: u8) -> Result<()> {
        let mint_ctx = CpiContext::new(
            self.token_program.to_account_info(),
            MintToChecked {
                authority: self.user.to_account_info(),
                mint: self.mint.to_account_info(),
                to: self.source_token_account.to_account_info(),
            },
        );

        mint_to_checked(mint_ctx, amount, decimals)?;

        Ok(())
    }
}
