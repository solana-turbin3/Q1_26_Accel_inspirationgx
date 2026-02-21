use anchor_lang::prelude::*;

#[error_code]
pub enum VaultError {
    #[msg("You can't recreate a vault")]
    VaultAlreadyExist,
    #[msg("Wrong Admin Provided")]
    NotAdmin,
    #[msg("Cannot remove user, user does not exist")]
    UserNotExistInVec,
    #[msg("overflow at multiplication on mint")]
    MUltiplicationAtMint,
    #[msg("Admin should already have created this Vault")]
    VaultNotCreatedByAdmin,
    #[msg("You provided a wrong ATA for the account")]
    WrongATA,
    #[msg("You provided a wrong Mint for the Instruction")]
    WrongMint,
    #[msg("You provided a wrong Authority for the Mint")]
    WrongMintAuthority,
    #[msg("You cannot transfer more than you own")]
    InsufficientBalance,
    #[msg("user does not exist in vec for real")]
    UserNotExistInVecForReal,
    #[msg("overflow at increasing user amount")]
    AdditionAtUpdateUserOverflow,
    #[msg("underflow at subtracting user amount")]
    SubtractionAtUpdateUserUnderflow,
}
