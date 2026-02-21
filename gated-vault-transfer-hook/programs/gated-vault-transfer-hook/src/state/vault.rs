use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Vault {
    pub mint: Pubkey,
    pub vault_bump: u8,
    pub owner: Pubkey,
}
