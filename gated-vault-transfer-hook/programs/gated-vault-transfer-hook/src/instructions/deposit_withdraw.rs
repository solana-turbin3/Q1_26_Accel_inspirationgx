
use anchor_lang::{prelude::*, solana_program::program::{invoke, invoke_signed}};
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{ Mint, TokenAccount, TokenInterface, TransferChecked, transfer_checked}
};
use spl_token_2022::{onchain,instruction as token_instruction};


use crate::{error::VaultError, Vault, Whitelist};
pub const VAULT_SEED: &[u8] = b"vault";

#[derive(Accounts)]
pub struct DepositWithdraw<'info> {
    #[account(mut)]
    pub sender: Signer<'info>,

    /// CHECK: owner is admin of platform
    #[account(mut)]
    pub owner: UncheckedAccount<'info>,

    #[account(
        mut,
        constraint = mint.mint_authority == Some(owner.key()).into() @VaultError::WrongMintAuthority,
        extensions::transfer_hook::program_id = hook_program_id.key(), 
    )]
    pub mint: InterfaceAccount<'info, Mint>,

    /// CHECK: Program id of the tf hook
    pub hook_program_id: UncheckedAccount<'info>,

    #[account(
        mut @VaultError::VaultNotCreatedByAdmin,
        seeds = [mint.key().as_ref(),VAULT_SEED],
        bump = vault_state.vault_bump
    )]
    pub vault_state: Account<'info, Vault>,

    #[account(
        mut,
        constraint = vault_ata.mint == mint.key() @VaultError::WrongMint        
    )]
    pub vault_ata: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        constraint = user_ata.mint == mint.key() @VaultError::WrongMint        
    )]
    pub user_ata: InterfaceAccount<'info, TokenAccount>,

     #[account(
        mut,
        seeds = [b"whitelist"],
        bump = whitelist.whitelist_bump,
    )]
    pub whitelist: Account<'info, Whitelist>,
    /// CHECK: Account containing the extra account
    pub extra_account_meta_list: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

impl<'info> DepositWithdraw<'info> {
    pub fn deposit(&mut self, amount: u64, remaining_accounts: &[AccountInfo<'info>]) -> Result<()> {

        require!(self.user_ata.amount >= amount,VaultError::InsufficientBalance);

        //   1. Manually build the `transfer_checked` instruction provided by the SPL Token program.
        let mut transfer_ix = token_instruction::transfer_checked(
            &self.token_program.key(),
            &self.user_ata.key(),
            &self.mint.key(),
            &self.vault_ata.key(),
            &self.sender.key(),
            &[], // No multisig signers are needed.
            amount,
            self.mint.decimals,
        )?;

        // // 2. Manually add the extra accounts required by the transfer hook.
        // // The Token 2022 program expects these to follow the core transfer accounts.
        transfer_ix.accounts.push(AccountMeta::new_readonly(self.extra_account_meta_list.key(), false));
        transfer_ix.accounts.push(AccountMeta::new(self.whitelist.key(), false));
        

        // // 3. Create a flat list of all AccountInfos that the instruction needs.
        // // This includes all accounts for the core transfer and the hook.
        let account_infos = &[
            self.user_ata.to_account_info(),
            self.mint.to_account_info(),
            self.vault_ata.to_account_info(),
            self.sender.to_account_info(),
            self.token_program.to_account_info(), // The Token Program must be in this list for `invoke`
            self.extra_account_meta_list.to_account_info(),
            self.whitelist.to_account_info(),
            self.hook_program_id.to_account_info(),
        ];

        // // 4. Use the low-level `invoke` function to execute the CPI.
        invoke(&transfer_ix, account_infos)?; 

        // transfer_checked(transfer_ctx, amount, self.mint.decimals)?;
        let user_key = self.user_ata.key();
    
    // Try to find existing entry
    if let Some(entry) = self.whitelist.address.iter_mut().find(|(addr, _, _)| *addr == user_key) {
        // User exists, add to their existing amount
        entry.1 = entry.1.checked_add(amount)
            .ok_or(VaultError::AdditionAtUpdateUserOverflow)?;
    } 
        Ok(())
    }

    pub fn withdraw(&mut self, amount: u64, remaining_accounts: &[AccountInfo<'info>]) -> Result<()> {

        // 1. Initial requirement checks
        let is_user_exist = self.whitelist.contains_address(&self.user_ata.key());
        require!(is_user_exist, VaultError::UserNotExistInVecForReal);

        let user_deposited_amount = self.whitelist.address
            .iter()
            .find(|(addr, _, _)| *addr == self.user_ata.key())
            .map(|(_, amount, _)| *amount)
            .unwrap_or(0);
        require!(user_deposited_amount >= amount, VaultError::InsufficientBalance);

        // 2. Build the `transfer_checked` instruction for withdrawal
        let mut transfer_ix = token_instruction::transfer_checked(
            &self.token_program.key(),
            &self.vault_ata.key(),      // Source
            &self.mint.key(),           // Mint
            &self.user_ata.key(),       // Destination
            &self.vault_state.key(),    // Authority (our PDA)
            &[],                        // No multi-sig
            amount,
            self.mint.decimals,
        )?;

        // 3. Add extra accounts for the transfer hook
        transfer_ix.accounts.push(AccountMeta::new_readonly(self.extra_account_meta_list.key(), false));
        transfer_ix.accounts.push(AccountMeta::new(self.whitelist.key(), false));
        

        // 4. Create the list of AccountInfos for the CPI
        let account_infos = &[
            self.vault_ata.to_account_info(),
            self.mint.to_account_info(),
            self.user_ata.to_account_info(),
            self.vault_state.to_account_info(), // The PDA authority
            self.token_program.to_account_info(),
            self.extra_account_meta_list.to_account_info(),
            self.whitelist.to_account_info(),
            self.hook_program_id.to_account_info(),
        ];

        // 5. Construct the PDA signer seeds
        let bump_slice = &[self.vault_state.vault_bump];
        let mint = self.mint.key();
        let seeds: &[&[u8]] = &[
            mint.as_ref(),
            &VAULT_SEED,
            bump_slice,
        ];
        let signer_seeds = &[&seeds[..]];

        // 6. Use `invoke_signed` to execute the CPI with PDA signature
        invoke_signed(&transfer_ix, account_infos, signer_seeds)?;
        msg!("Invoke signed transfer successful for withdraw!");

        // deduct amount from user vec vault balance

        let user_key = self.user_ata.key();
    if let Some(pos) = self.whitelist.address.iter().position(|(addr, _, _)| *addr == user_key) {
        let (_, user_balance, _) = &mut self.whitelist.address[pos];
        *user_balance = user_balance.checked_sub(amount)
            .ok_or(VaultError::SubtractionAtUpdateUserUnderflow)?;
        
        // Optionally remove entry if balance is 0
        if *user_balance == 0 {
            self.whitelist.address.remove(pos);
        }
    }

        Ok(())
    }

    // pub fn deposit(&mut self, amount: u64)
}
