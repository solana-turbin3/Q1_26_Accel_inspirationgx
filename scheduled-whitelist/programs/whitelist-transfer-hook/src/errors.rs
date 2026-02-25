use anchor_lang::error_code;

#[error_code]
pub enum WhitelistError {
    #[msg("Whitelist not Initialized")]
    NotInitialized,
    #[msg("Wrong Account for Whitelist operation")]
    WrongAccount,
}
