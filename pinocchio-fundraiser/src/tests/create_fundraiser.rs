#[cfg(test)]
pub mod create_fundraiser {
    use core::fmt::Error;

    use crate::instructions::{FundraiserInstruction, InitData};
    use crate::tests::tests::{program_id, ReusableState};
    use litesvm::LiteSVM;
    use solana_instruction::{AccountMeta, Instruction};
    use solana_message::Message;
    use solana_sdk_ids::sysvar::rent;
    use solana_signer::Signer;
    use solana_transaction::Transaction;

    pub fn create_fundraiser_function(
        svm: &mut LiteSVM,
        state: &ReusableState,
    ) -> Result<(), Error> {
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

        let min_amount_to_donate: u64 = 10_000_000; // 10 usdc - 6 decimal places
        let max_amount_to_donate: u64 = 4000_000_000; // 500 tokens with 6 decimal places
        let amount_to_raise: u64 = 10_000_000_000; // 10k usdc - 6 decimals
        let bump: u8 = fundraiser.1;

        println!("Bump: {}", bump);

        let rent_sysvar = rent::ID;

        // Create the "Make" instruction to deposit tokens into the escrow
        // let initilize_data = [
        //     vec![0u8], // Discriminator for "Make" instruction
        //     bump.to_le_bytes().to_vec(),
        //     amount_to_receive.to_le_bytes().to_vec(),
        //     amount_to_give.to_le_bytes().to_vec(),
        // ]
        // .concat();

        let initialize_data = InitData {
            amount_to_raise: amount_to_raise.to_le_bytes(),
            duration: 3u8.to_le_bytes(),
            max_amount_sendable: max_amount_to_donate.to_le_bytes(),
            min_amount_sendable: min_amount_to_donate.to_le_bytes(),
        };

        let make_ix = Instruction {
            program_id: program_id(),
            accounts: vec![
                AccountMeta::new(maker.pubkey(), true),
                AccountMeta::new(*mint, false),
                AccountMeta::new(fundraiser.0, false),
                AccountMeta::new(*vault, false),
                AccountMeta::new(*system_program, false),
                AccountMeta::new(*token_program, false),
                AccountMeta::new(*ata_program, false),
                AccountMeta::new(rent_sysvar, false),
            ],
            data: [
                (FundraiserInstruction::Initialize as u8)
                    .to_le_bytes()
                    .to_vec(),
                initialize_data.to_bytes().to_vec(),
            ]
            .concat(),
        };

        // Create and send the transaction containing the "Make" instruction
        let message = Message::new(&[make_ix], Some(&maker.pubkey()));
        let recent_blockhash = svm.latest_blockhash();

        let transaction = Transaction::new(&[&maker], message, recent_blockhash);

        // Send the transaction and capture the result
        let tx = svm.send_transaction(transaction).unwrap();

        // Log transaction details
        println!("\nInitialize transaction sucessfull");
        println!("CUs Consumed: {}", tx.compute_units_consumed);
        Ok(())
    }
}
