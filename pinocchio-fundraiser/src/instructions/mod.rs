pub mod contribute;
pub mod initialize;

pub use contribute::*;
pub use initialize::*;
use pinocchio::error::ProgramError;

pub enum FundraiserInstruction {
    Initialize = 0,
    Deposit = 1,
    // Cancel = 2,
    // MakeV2 = 3,
}

impl TryFrom<&u8> for FundraiserInstruction {
    type Error = ProgramError;

    fn try_from(value: &u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(FundraiserInstruction::Initialize),
            1 => Ok(FundraiserInstruction::Deposit),
            // 2 => Ok(FundraiserInstruction::Cancel),
            // 3 => Ok(FundraiserInstruction::MakeV2),
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }
}
