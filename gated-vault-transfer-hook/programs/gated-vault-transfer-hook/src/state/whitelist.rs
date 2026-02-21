use anchor_lang::prelude::*;

#[account]
// #[derive(InitSpace)] can't use initspace since i'm using a dynamic vec
pub struct Whitelist {
    pub address: Vec<(Pubkey, u64, bool)>,
    pub whitelist_bump: u8,
    pub admin: Pubkey,
}

impl Whitelist {
    pub fn contains_address(&self, address: &Pubkey) -> bool {
        self.address.iter().any(|(addr, _, _)| addr == address)
    }

    pub fn is_whitelisted(&self, address: &Pubkey) -> Option<&bool> {
        let user_is_whitelisted = self
            .address
            .iter()
            .find(|(addr, _, _)| *address == *addr)
            .map(|(_, _, is_whitelisted)| is_whitelisted);
        user_is_whitelisted
    }
}
