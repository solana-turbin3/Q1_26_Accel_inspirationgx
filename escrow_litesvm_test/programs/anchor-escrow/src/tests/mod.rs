#[cfg(test)]
mod tests {

    use {
        anchor_lang::{
            // accounts::program,
            prelude::msg,
            solana_program::program_pack::Pack,
            AccountDeserialize,
            InstructionData,
            ToAccountMetas,
        },
        anchor_spl::{
            associated_token::{self, spl_associated_token_account},
            token::spl_token,
        },
        litesvm::LiteSVM,
        litesvm_token::{
            spl_token::ID as TOKEN_PROGRAM_ID, CreateAssociatedTokenAccount, CreateMint, MintTo,
        },
        // solana_account::Account,
        // solana_address::Address,
        solana_clock::Clock,
        solana_instruction::Instruction,
        solana_keypair::Keypair,
        solana_message::Message,
        solana_native_token::LAMPORTS_PER_SOL,
        solana_pubkey::Pubkey,
        // solana_rpc_client::rpc_client::RpcClient,
        solana_sdk_ids::system_program::ID as SYSTEM_PROGRAM_ID,
        solana_signer::Signer,
        solana_transaction::Transaction,
        std::{path::PathBuf, str::FromStr},
    };

    #[derive(Debug)]
    pub struct ReusableState {
        // pub program: LiteSVM,
        pub global_signer: Keypair,
        pub mint_a: Pubkey,
        mint_b: Pubkey,
        maker_ata_b: Pubkey,
        escrow: Pubkey,
        vault: Pubkey,
        maker: Pubkey,
        maker_ata_a: Pubkey,
    }

    static PROGRAM_ID: Pubkey = crate::ID;

    fn setup() -> (LiteSVM, ReusableState) {
        // Initialize LiteSVM and payer
        let mut program = LiteSVM::new();
        let payer = Keypair::new();

        // let maker = &payer.pubkey();

        // Airdrop some SOL to the payer keypair
        program
            .airdrop(&payer.pubkey(), 10 * LAMPORTS_PER_SOL)
            .expect("Failed to airdrop SOL to payer");

        // Load program SO file
        let so_path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/deploy/anchor_escrow.so");

        let program_data = std::fs::read(so_path).expect("Failed to read program SO file");

        program.add_program(PROGRAM_ID, &program_data);

        // Example on how to Load an account from devnet
        // let rpc_client = RpcClient::new("https://api.devnet.solana.com");
        // let account_address =
        //     Address::from_str("CzsDexXbnEdQbQtwos29vushaNQ3Ddf9Wy6J85KD34N5").unwrap();

        // let fetched_account = rpc_client
        //     .get_account(&account_address)
        //     .expect("Failed to fetch account from devnet");

        // // see if it's a System Account
        // // msg!("Fetched Account Data: {:#?}", fetched_account.data);

        // program
        //     .set_account(
        //         payer.pubkey(),
        //         Account {
        //             lamports: fetched_account.lamports,
        //             data: fetched_account.data,
        //             owner: Pubkey::from(fetched_account.owner.to_bytes()),
        //             executable: fetched_account.executable,
        //             rent_epoch: fetched_account.rent_epoch,
        //         },
        //     )
        //     .unwrap();

        // msg!("Lamports of fetched account: {}", fetched_account.lamports);

        let maker = payer.pubkey();

        let mint_a = CreateMint::new(&mut program, &payer)
            .decimals(6)
            .authority(&maker)
            .send()
            .unwrap();
        msg!("Mint A: {}\n", mint_a);

        let mint_b = CreateMint::new(&mut program, &payer)
            .decimals(6)
            .authority(&maker)
            .send()
            .unwrap();
        msg!("Mint B: {}\n", mint_b);

        // Create the maker's associated token account for Mint A
        let maker_ata_a = CreateAssociatedTokenAccount::new(&mut program, &payer, &mint_a)
            .owner(&maker)
            .send()
            .unwrap();
        msg!("Maker ATA A: {}\n", maker_ata_a);

        let maker_ata_b = CreateAssociatedTokenAccount::new(&mut program, &payer, &mint_b)
            .owner(&maker)
            .send()
            .unwrap();
        msg!("Maker ATA B: {}\n", maker_ata_b);

        // Derive the PDA for the escrow account using the maker's public key and a seed value
        let escrow = Pubkey::find_program_address(
            &[b"escrow", maker.as_ref(), &123u64.to_le_bytes()],
            &PROGRAM_ID,
        )
        .0;
        msg!("Escrow PDA: {}\n", escrow);

        // Derive the PDA for the vault associated token account using the escrow PDA and Mint A
        let vault = associated_token::get_associated_token_address(&escrow, &mint_a);
        msg!("Vault PDA: {}\n", vault);

        // Return the LiteSVM instance and payer keypair
        // (program, payer)

        (
            program,
            ReusableState {
                global_signer: payer,
                mint_a,
                mint_b,
                maker_ata_b,
                escrow,
                vault,
                maker,
                maker_ata_a,
            },
        )
    }

    #[test] // returns mint_a, mint_b, maker_ata_b, escrow, vault,maker
    fn test_make() {
        let (program, reusable_data) = &mut setup();

        msg!("{:#?}", reusable_data);
        // let {mut program, payer} = setup();

        let mint_a = reusable_data.mint_a;
        let mint_b = reusable_data.mint_b;
        let maker_ata_a = reusable_data.maker_ata_a;
        let escrow = reusable_data.escrow;
        let vault = reusable_data.vault;
        let payer = &mut reusable_data.global_signer;
        let program = program;

        // Get the maker's public key from the payer keypair
        let maker = payer.pubkey();

        // Create two mints (Mint A and Mint B) with 6 decimal places and the maker as the authority

        // Define program IDs for associated token program, token program, and system program
        let asspciated_token_program = spl_associated_token_account::ID;
        let token_program = TOKEN_PROGRAM_ID;
        let system_program = SYSTEM_PROGRAM_ID;

        // Mint 1,000 tokens (with 6 decimal places) of Mint A to the maker's associated token account
        MintTo::new(program, &payer, &mint_a, &maker_ata_a, 1000000000)
            .send()
            .unwrap();

        let initial_clock = program.get_sysvar::<Clock>();
        let current_time = initial_clock.unix_timestamp;

        // disable taking for two days

        let two_days_in_seconds = 2 * 24 * 60 * 60;
        let lock_period = current_time + two_days_in_seconds;

        // Create the "Make" instruction to deposit tokens into the escrow
        let make_ix = Instruction {
            program_id: PROGRAM_ID,
            accounts: crate::accounts::Make {
                maker: maker,
                mint_a: mint_a,
                mint_b: mint_b,
                maker_ata_a: maker_ata_a,
                escrow: escrow,
                vault: vault,
                associated_token_program: asspciated_token_program,
                token_program: token_program,
                system_program: system_program,
            }
            .to_account_metas(None),
            data: crate::instruction::Make {
                deposit: 10,
                seed: 123u64,
                receive: 10,
                lock_period,
            }
            .data(),
        };

        // Create and send the transaction containing the "Make" instruction
        let message = Message::new(&[make_ix], Some(&payer.pubkey()));
        let recent_blockhash = program.latest_blockhash();

        let transaction = Transaction::new(&[&payer], message, recent_blockhash);

        // Send the transaction and capture the result
        let tx = program.send_transaction(transaction).unwrap();

        // Log transaction details
        msg!("\n\nMake transaction sucessfull");
        msg!("CUs Consumed: {}", tx.compute_units_consumed);
        msg!("Tx Signature: {}", tx.signature);

        // Verify the vault account and escrow account data after the "Make" instruction
        let vault_account = program.get_account(&vault).unwrap();
        let vault_data = spl_token::state::Account::unpack(&vault_account.data).unwrap();
        assert_eq!(vault_data.amount, 10);
        assert_eq!(vault_data.owner, escrow);
        assert_eq!(vault_data.mint, mint_a);

        let escrow_account = program.get_account(&escrow).unwrap();
        let escrow_data =
            crate::state::Escrow::try_deserialize(&mut escrow_account.data.as_ref()).unwrap();
        assert_eq!(escrow_data.seed, 123u64);
        assert_eq!(escrow_data.maker, maker);
        assert_eq!(escrow_data.mint_a, mint_a);
        assert_eq!(escrow_data.mint_b, mint_b);
        assert_eq!(escrow_data.receive, 10);
    }

    #[test]
    fn test_take() {
        // // Setup the test environment by initializing LiteSVM and creating a payer keypair

        let (program, reusable_data) = &mut setup();

        let mint_a = reusable_data.mint_a;
        let mint_b = reusable_data.mint_b;
        let maker_ata_a = reusable_data.maker_ata_a;
        let maker_ata_b = reusable_data.maker_ata_b;
        let escrow = reusable_data.escrow;
        let vault = reusable_data.vault;
        let payer = &mut reusable_data.global_signer;
        let program = program;

        // Get the maker's public key from the payer keypair
        let maker = payer.pubkey();
        let taker = Keypair::new();

        let token_program = TOKEN_PROGRAM_ID;
        let system_program = SYSTEM_PROGRAM_ID;

        // make instruction
        MintTo::new(program, &payer, &mint_a, &maker_ata_a, 1000000000)
            .send()
            .unwrap();

        let associated_token_program = spl_associated_token_account::ID;

        let mut initial_clock = program.get_sysvar::<Clock>();
        let current_time = initial_clock.unix_timestamp;

        // disable taking for two days

        let two_days_in_seconds = 2 * 24 * 60 * 60;
        let lock_period = current_time + two_days_in_seconds;

        // Create the "Make" instruction to deposit tokens into the escrow
        let make_ix = Instruction {
            program_id: PROGRAM_ID,
            accounts: crate::accounts::Make {
                maker: maker,
                mint_a: mint_a,
                mint_b: mint_b,
                maker_ata_a: maker_ata_a,
                escrow: escrow,
                vault: vault,
                associated_token_program: associated_token_program,
                token_program: token_program,
                system_program: system_program,
            }
            .to_account_metas(None),
            data: crate::instruction::Make {
                deposit: 10,
                seed: 123u64,
                receive: 10,
                lock_period,
            }
            .data(),
        };

        // Create and send the transaction containing the "Make" instruction
        let message = Message::new(&[make_ix], Some(&payer.pubkey()));
        let recent_blockhash = program.latest_blockhash();

        let transaction = Transaction::new(&[&payer], message, recent_blockhash);

        // Send the transaction and capture the result
        let _tx_make = program.send_transaction(transaction).unwrap();

        program.airdrop(&taker.pubkey(), 5_000_000_000).unwrap();
        let taker_ata_a = CreateAssociatedTokenAccount::new(program, &taker, &mint_a)
            .owner(&taker.pubkey())
            .send()
            .unwrap();

        let taker_ata_b = CreateAssociatedTokenAccount::new(program, &taker, &mint_b)
            .owner(&taker.pubkey())
            .send()
            .unwrap();

        // Mint 1,000 tokens (with 6 decimal places) of Mint B to the maker's associated token account
        MintTo::new(program, &payer, &mint_b, &taker_ata_b, 1000000000)
            .send()
            .unwrap();

        // jump forward in time
        initial_clock.unix_timestamp = current_time + two_days_in_seconds;
        program.set_sysvar::<Clock>(&initial_clock);

        let take_ix = Instruction {
            program_id: PROGRAM_ID,
            accounts: crate::accounts::Take {
                associated_token_program: spl_associated_token_account::ID,
                token_program,
                system_program,
                taker: taker.pubkey(),
                mint_a,
                mint_b,
                escrow,
                vault,
                maker_ata_b,
                maker,
                taker_ata_a,
                taker_ata_b,
            }
            .to_account_metas(None),
            data: crate::instruction::Take {}.data(),
        };

        // Create and send the transaction containing the "Make" instruction
        let message = Message::new(&[take_ix], Some(&taker.pubkey()));
        let recent_blockhash = program.latest_blockhash();

        let transaction = Transaction::new(&[&taker], message, recent_blockhash);

        // Send the transaction and capture the result
        let _tx_take = program.send_transaction(transaction).unwrap();
    }
}
