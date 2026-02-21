use anchor_lang::{prelude::*, system_program};
use anchor_spl::token_interface::spl_pod::option::Nullable;

use crate::{state::whitelist::Whitelist, Vault};

use crate::error::VaultError;
use crate::instructions::VAULT_SEED;

#[derive(Accounts)]
#[instruction(address: Pubkey, mint: Pubkey)]
pub struct WhitelistOperations<'info> {
    #[account(
        mut,
        constraint = admin.key() == vault.owner.key() @VaultError::NotAdmin
    )]
    pub admin: Signer<'info>,
    #[account(
        init_if_needed,
        payer = admin,
        seeds = [b"whitelist"],
        space = 8 + 4 + (1 * 32 + 8 + 1) + 1, // 8 bytes for discriminator, 4 bytes for vector length, (1 * 32 + 8) => 1 length_of_vec * 32 pubkey_space + 8 u64_amount, 1 byte for bump  // but this is redundant since I'm allocating the size dynamically anyway, 8 + 4 + 1 would do anyway
        bump,
    )]
    pub whitelist: Account<'info, Whitelist>,

    #[account(
        mut @VaultError::VaultNotCreatedByAdmin,
        seeds = [mint.key().as_ref(), VAULT_SEED],
        bump = vault.vault_bump,        
    )]
    pub vault: Account<'info, Vault>,
    pub system_program: Program<'info, System>,
}

impl<'info> WhitelistOperations<'info> {
    pub fn add_to_whitelist(&mut self, address: Pubkey, bumps: &WhitelistOperationsBumps) -> Result<()> {

        
        let whitelist_accounts = &mut self.whitelist;

        // create the whitelist state if it does not exist
        if Pubkey::is_none(&whitelist_accounts.admin) {
            whitelist_accounts.admin = self.admin.key();
            whitelist_accounts.whitelist_bump = bumps.whitelist;
        }

        if !whitelist_accounts.contains_address(&address) {
            self.realloc_whitelist(true)?;
            self.whitelist.address.push((address, 0, true));
        }
        Ok(())
    }

    pub fn remove_from_whitelist(&mut self, address: Pubkey) -> Result<()> {

        let whitelist_accounts = &mut self.whitelist;
        require!(
            whitelist_accounts.contains_address(&address),
            VaultError::UserNotExistInVec
        );
        if let Some(pos) = self
            .whitelist
            .address
            .iter()
            .position(|(addr, _, _)| *addr == address)
        {
            self.whitelist.address.remove(pos);
            self.realloc_whitelist(false)?;
        }
        Ok(())
    }

    pub fn realloc_whitelist(&self, is_adding: bool) -> Result<()> {
        // Get the account info for the whitelist
        let account_info = self.whitelist.to_account_info();

        // Size of one whitelist entry: Pubkey (32 bytes) + u64 (8 bytes)
        const WHITELIST_ENTRY_SIZE: usize =
            std::mem::size_of::<Pubkey>() + std::mem::size_of::<u64>() + 1;

        if is_adding {
            // Adding to whitelist
            let new_account_size = account_info.data_len() + WHITELIST_ENTRY_SIZE;
            // Calculate rent required for the new account size
            let lamports_required = (Rent::get()?).minimum_balance(new_account_size);
            // Determine additional rent required
            let rent_diff = lamports_required - account_info.lamports();

            // Perform transfer of additional rent
            let cpi_program = self.system_program.to_account_info();
            let cpi_accounts = system_program::Transfer {
                from: self.admin.to_account_info(),
                to: account_info.clone(),
            };
            let cpi_context = CpiContext::new(cpi_program, cpi_accounts);
            system_program::transfer(cpi_context, rent_diff)?;

            // Reallocate the account
            account_info.resize(new_account_size)?;
            msg!("Account Size Updated: {}", account_info.data_len());
        } else {
            // Removing from whitelist
            let new_account_size = account_info.data_len() - WHITELIST_ENTRY_SIZE;
            // Calculate rent required for the new account size
            let lamports_required = (Rent::get()?).minimum_balance(new_account_size);
            // Determine additional rent to be refunded
            let rent_diff = account_info.lamports() - lamports_required;

            // Reallocate the account
            account_info.resize(new_account_size)?;
            msg!("Account Size Downgraded: {}", account_info.data_len());

            // Perform transfer to refund additional rent
            **self.admin.to_account_info().try_borrow_mut_lamports()? += rent_diff;
            **self.whitelist.to_account_info().try_borrow_mut_lamports()? -= rent_diff;
        }

        Ok(())
    }

    
}