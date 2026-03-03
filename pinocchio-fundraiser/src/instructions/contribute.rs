use core::cmp::Ordering;

use pinocchio::{
    cpi::{Seed, Signer},
    error::ProgramError,
    sysvars::{clock::Clock, rent::Rent, Sysvar},
    AccountView, Address, ProgramResult,
};

use pinocchio_log::log;
use pinocchio_system::instructions::CreateAccount;
use pinocchio_token::{instructions::Transfer, state::TokenAccount};

use crate::state::{Contributor, Fundraiser};

pub fn process_contribute_instruction(accounts: &[AccountView], data: &[u8]) -> ProgramResult {
    let [contributor, mint_to_raise, fundraiser, contributor_state_account, contributor_ata, vault, system_program, token_program, associated_token_program, rent_sysvar, _extra @ ..] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    let parsed_data = bytemuck::from_bytes::<Contributor>(&data[..Contributor::LEN]);

    // ensure contributor is signer

    assert!(contributor.is_signer(), "Contributor must be signer");

    let fundraiser_data = fundraiser.try_borrow().unwrap();

    // ensure fundraiser exists and was created with this program_id
    let fundraiser_as_state_account = bytemuck::from_bytes::<Fundraiser>(&*fundraiser_data);

    assert!(!fundraiser.is_data_empty(), "Fundraiser must exist");
    assert!(
        fundraiser.owned_by(&crate::ID),
        "Invalid Fundraiser Account"
    );

    // ensure time is still valid for contribution
    let fundraising_ends_days = u8::from_le_bytes(fundraiser_as_state_account.duration);
    let unix_time_fundraising_started =
        u64::from_le_bytes(fundraiser_as_state_account.time_started);

    let current_time_unix = Clock::get()?.unix_timestamp;

    let target_days_in_unix: u64 = u64::from(fundraising_ends_days) * 24 * 60 * 60;

    let expiration_time = target_days_in_unix + unix_time_fundraising_started;

    assert!(
        expiration_time > current_time_unix as u64,
        "Fundraising closed"
    );

    // ensure contributor ata exists - scoping to drop contributot_ata once done
    {
        let contributor_ata_as_state = TokenAccount::from_account_view(contributor_ata)
            .map_err(|_| "Token Acccount does not exist")
            .unwrap();

        assert!(
            contributor_ata_as_state.amount() > u64::from_le_bytes(parsed_data.amount),
            "insufficient contributor balance"
        );
    }

    // ensure mint to raise is same as one stored in fundraiser state

    let compared_mint = mint_to_raise
        .address()
        .partial_cmp(&Address::new_from_array(
            fundraiser_as_state_account.mint_to_raise,
        ));
    assert_eq!(compared_mint, Some(Ordering::Equal), "mint do not match");

    // compared another way 👀👀 10 CUs cheaper this way
    // assert!(
    //     mint_to_raise.address().eq(&Address::new_from_array(
    //         fundraiser_as_state_account.mint_to_raise,
    //     )),
    //     "mint do not match"
    // );

    // assert vault provided is correct
    assert!(
        vault
            .address()
            .eq(&Address::new_from_array(fundraiser_as_state_account.vault,)),
        "vault do not match"
    );

    // transfer to vault
    Transfer {
        amount: u64::from_le_bytes(parsed_data.amount),
        authority: &contributor,
        from: &contributor_ata,
        to: &vault,
    }
    .invoke()?;
    log!("got here 👀");

    let rent = Rent::get()?;
    let minimum_balance = rent.minimum_balance_unchecked(Contributor::LEN);

    let seed = [b"contributor", contributor.address().as_ref()];
    let (created_contributor, contributor_bump) = Address::find_program_address(&seed, &crate::ID);

    let bump = contributor_bump.to_le_bytes();

    let contributor_seeds = [
        Seed::from(b"contributor"),
        Seed::from(contributor.address().as_ref()),
        Seed::from(&bump),
    ];

    assert!(
        &created_contributor.eq(contributor_state_account.address()),
        "Contributor does not match"
    );

    // if contributor state doesn't exist, create it
    if contributor_state_account.is_data_empty() {
        CreateAccount {
            from: &contributor,
            lamports: minimum_balance,
            owner: &crate::ID,
            space: Contributor::LEN as u64,
            to: contributor_state_account,
        }
        .invoke_signed(&[Signer::from(&contributor_seeds)])?;
    }

    // modify contributor state amount
    let mut contributor_state_data = contributor_state_account.try_borrow_mut().unwrap();

    let contributor_mutable = bytemuck::from_bytes_mut::<Contributor>(&mut contributor_state_data);

    let current_amount = u64::from_le_bytes(contributor_mutable.amount);
    let total_amount = current_amount + u64::from_le_bytes(parsed_data.amount);

    contributor_mutable.amount = total_amount.to_le_bytes();

    Ok(())
}
