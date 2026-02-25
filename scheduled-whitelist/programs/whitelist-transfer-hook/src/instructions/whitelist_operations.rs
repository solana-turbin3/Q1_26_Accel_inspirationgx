use anchor_lang::{prelude::*, system_program};
use anchor_spl::token_interface::TokenAccount;

use crate::{errors::WhitelistError, state::whitelist::Whitelist};

#[derive(Accounts)]
pub struct WhitelistOperations<'info> {
    #[account(
        mut,
        //address = 
    )]
    pub admin: Signer<'info>,
    #[account(
        mut @WhitelistError::NotInitialized,
        seeds = [b"whitelist", user_wallet.key().as_ref()],
        bump,
    )]
    pub whitelist: Account<'info, Whitelist>,

    #[account(
        mut,
        constraint = user_wallet.key() == whitelist.address.key() @WhitelistError::WrongAccount
    )]
    pub user_wallet: SystemAccount<'info>,
    pub system_program: Program<'info, System>,
}

impl<'info> WhitelistOperations<'info> {
    pub fn add_to_whitelist(&mut self, user_wallet: Pubkey) -> Result<()> {
        assert_eq!(
            &user_wallet,
            &self.user_wallet.key(),
            "user wallet mismatch"
        );

        if self.whitelist.is_whitelisted {
            panic!("User is already whitelisted");
        }
        self.whitelist.is_whitelisted = true;
        Ok(())
    }

    pub fn remove_from_whitelist(&mut self, user_wallet: Pubkey) -> Result<()> {
        assert_eq!(
            &user_wallet,
            &self.user_wallet.key(),
            "user wallet mismatch"
        );
        if !self.whitelist.is_whitelisted {
            panic!("User is not whitelisted but account exists");
        }
        self.whitelist.is_whitelisted = false;
        Ok(())
    }

    pub fn close_whitelist(&mut self) -> Result<()> {
        self.whitelist.close(self.user_wallet.to_account_info())?;
        Ok(())
    }
}
