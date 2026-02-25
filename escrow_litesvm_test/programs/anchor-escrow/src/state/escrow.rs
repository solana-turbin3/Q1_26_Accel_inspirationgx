use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace, Debug)]
pub struct Escrow {
    pub seed: u64,
    pub maker: Pubkey,
    pub mint_a: Pubkey, // wSOL
    pub mint_b: Pubkey, // USDC
    pub receive: u64,   // amount of USDC
    pub lock_period: i64,
    pub bump: u8,
}
