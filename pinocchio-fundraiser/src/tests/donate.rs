#[cfg(test)]
pub mod donate {
    use core::fmt::Error;

    use crate::instructions::{FundraiserInstruction, InitData};
    use crate::state::Contributor;
    use crate::tests::tests::{program_id, ReusableState};
    use litesvm::LiteSVM;
    use litesvm_token::{CreateAssociatedTokenAccount, MintTo};
    use solana_instruction::{AccountMeta, Instruction};
    use solana_keypair::Keypair;
    use solana_message::Message;
    use solana_native_token::LAMPORTS_PER_SOL;
    use solana_sdk_ids::{system_program, sysvar::rent};
    use solana_signer::Signer;
    use solana_transaction::Transaction;

    pub fn donate_function(svm: &mut LiteSVM, state: &ReusableState) -> Result<(), Error> {
        let ReusableState {
            maker,
            fundraiser,
            mint,
            vault,
            system_program,
            token_program,
            ata_program,
            ..
        } = state;

        let amount_to_donate: u64 = 100_000_000; // 100 usdc - 6 decimals
        let bump: u8 = fundraiser.1;

        println!("Bump: {}", bump);

        let rent_sysvar = rent::ID;

        let contributor = Keypair::new();

        svm.airdrop(&contributor.pubkey(), 10 * LAMPORTS_PER_SOL)
            .expect("Airdrop failed");

        let contributor_ata = CreateAssociatedTokenAccount::new(svm, &contributor, &mint)
            .owner(&contributor.pubkey())
            .send()
            .unwrap();

        MintTo::new(svm, &maker, &mint, &contributor_ata, 1_000_000_000)
            .send()
            .unwrap();

        let contributor_pda = solana_pubkey::Pubkey::find_program_address(
            &[b"contributor".as_ref(), contributor.pubkey().as_ref()],
            &program_id(),
        );

        let initialize_data = Contributor {
            amount: amount_to_donate.to_le_bytes(),
        };

        let donate_ix = Instruction {
            program_id: program_id(),
            accounts: vec![
                AccountMeta::new(contributor.pubkey(), true),
                AccountMeta::new(*mint, false),
                AccountMeta::new(fundraiser.0, false),
                AccountMeta::new(contributor_pda.0, false),
                AccountMeta::new(contributor_ata, false),
                AccountMeta::new(*vault, false),
                AccountMeta::new(*system_program, false),
                AccountMeta::new(*token_program, false),
                AccountMeta::new(*ata_program, false),
                AccountMeta::new(rent_sysvar, false),
            ],
            data: [
                (FundraiserInstruction::Deposit as u8)
                    .to_le_bytes()
                    .to_vec(),
                initialize_data.to_bytes().to_vec(),
            ]
            .concat(),
        };

        // Create and send the transaction containing the "Make" instruction
        let message = Message::new(&[donate_ix], Some(&contributor.pubkey()));
        let recent_blockhash = svm.latest_blockhash();

        let transaction = Transaction::new(&[&contributor], message, recent_blockhash);

        // Send the transaction and capture the result
        let tx = svm.send_transaction(transaction).unwrap();

        // Log transaction details
        println!("\nDonate transaction sucessfull");
        println!("CUs Consumed: {}", tx.compute_units_consumed);
        Ok(())
    }
}
