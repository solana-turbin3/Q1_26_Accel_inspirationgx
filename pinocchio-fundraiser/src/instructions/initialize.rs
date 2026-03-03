use bytemuck::{Pod, Zeroable};
use pinocchio::{
    cpi::{Seed, Signer},
    error::ProgramError,
    sysvars::{self, rent::Rent, Sysvar},
    AccountView, Address, ProgramResult,
};
use pinocchio_associated_token_account::instructions::Create;

use pinocchio_system::instructions::CreateAccount;
use pinocchio_token::state::Mint;

use crate::state::Fundraiser;

#[repr(C, packed)]
#[derive(Pod, Zeroable, Clone, Copy)]
pub struct InitData {
    pub min_amount_sendable: [u8; 8],
    pub max_amount_sendable: [u8; 8],
    pub amount_to_raise: [u8; 8],
    pub duration: [u8; 1],
}

impl InitData {
    pub const LEN: usize = core::mem::size_of::<InitData>();

    pub fn to_bytes(&self) -> &[u8; Self::LEN] {
        bytemuck::bytes_of(self).try_into().unwrap()
    }
}

pub fn process_initialize_instruction(accounts: &[AccountView], data: &[u8]) -> ProgramResult {
    // load accounts
    let [maker, mint_to_raise, fundraiser, vault, system_program, token_program, associated_token_program, rent_sysvar, _extra @ ..] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    // check that maker is a signer
    assert!(maker.is_signer(), "Maker should be signer");

    // cast data to type
    let parsed_data = bytemuck::from_bytes::<InitData>(&data[..InitData::LEN]);

    // constraints
    // check that mint exists [similar to mut in ancor]
    let mint_as_state_account = Mint::from_account_view(mint_to_raise).unwrap();
    assert!(
        mint_as_state_account.is_initialized(),
        "Mint you passed does not exist"
    );

    // check that fundraiser is empty
    assert!(fundraiser.is_data_empty(), "Wrong Fundraiser");

    // check that vault is empty and is not initialized
    // let vault_as_state_account = TokenAccount::from_account_view(vault).unwrap();

    assert!(vault.is_data_empty(), "Vault is already initialized");

    let rent = Rent::get()?;
    let minimum_balance = rent.minimum_balance_unchecked(Fundraiser::LEN);

    let seed = [b"fundraiser", maker.address().as_ref()];
    let (created_fundraiser, fundraiser_bump) = Address::find_program_address(&seed, &crate::ID);

    let bump = fundraiser_bump.to_le_bytes();

    let pda_seeds = [
        Seed::from(b"fundraiser"),
        Seed::from(maker.address().as_ref()),
        Seed::from(&bump),
    ];

    // log!("{}", &*created_fundraiser.to_string());

    // log!("{}", &*fundraiser.address().to_string());
    // compare fundraiser accounts from client vs onchain
    assert!(
        &created_fundraiser.eq(fundraiser.address()),
        "Fundraiser does not match"
    );

    // Create Fundraiser Account
    CreateAccount {
        from: maker,
        lamports: minimum_balance,
        owner: &crate::ID,
        space: Fundraiser::LEN as u64,
        to: fundraiser,
    }
    .invoke_signed(&[Signer::from(&pda_seeds)])?;

    // log!("got here 🫵");

    // Create ATA for Fundraiser - we can decide to move this to client instead 👀 👀
    Create {
        funding_account: maker,
        system_program: system_program,
        token_program: token_program,
        wallet: fundraiser,
        account: vault,
        mint: mint_to_raise,
    }
    .invoke()?;

    // log!("got here 👀");
    // write to the created account
    let mut mut_borrow = fundraiser.try_borrow_mut().unwrap();

    let fundraiser_mutable = bytemuck::from_bytes_mut::<Fundraiser>(&mut mut_borrow);

    fundraiser_mutable.maker = maker.address().as_ref().try_into().unwrap();
    fundraiser_mutable.mint_to_raise = mint_to_raise.address().as_ref().try_into().unwrap();
    fundraiser_mutable.amount_to_raise = parsed_data.amount_to_raise;
    fundraiser_mutable.current_amount = 0u64.to_le_bytes();
    fundraiser_mutable.time_started = (sysvars::clock::Clock::get()?.unix_timestamp).to_le_bytes();
    fundraiser_mutable.duration = parsed_data.duration;
    fundraiser_mutable.bump = fundraiser_bump.to_le_bytes();
    fundraiser_mutable.vault = vault.address().as_ref().try_into().unwrap();

    Ok(())
}
