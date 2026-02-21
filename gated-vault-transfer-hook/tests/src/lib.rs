#[cfg(test)]
mod tests {

    use {
        anchor_lang::{
            error::Error,
            prelude::{msg, AccountMeta},
            solana_program::{
                hash::Hash, native_token::LAMPORTS_PER_SOL, program_pack::Pack, pubkey::Pubkey,
            },
            system_program::ID as SYSTEM_PROGRAM_ID,
            AccountDeserialize, InstructionData, Key, ToAccountMetas,
        },
        anchor_spl::{
            associated_token::{self, spl_associated_token_account},
            token_2022::spl_token_2022,
        },
        litesvm::LiteSVM,
        solana_address::Address,
        solana_hash::Hash as SolanaHash,
        solana_instruction::Instruction,
        solana_keypair::Keypair,
        solana_signer::Signer,
        solana_transaction::Transaction,
        std::{path::PathBuf, str::FromStr},
    };

    // trait PubkeyExt {
    //     fn to_address(&self) -> Address;
    //     fn to_pubkey(&self) -> Pubkey;
    // }

    // // Implement it for Pubkey
    // impl PubkeyExt for Pubkey {
    //     fn to_address(&self) -> Address {
    //         Address::from(self.to_bytes())
    //     }

    //     fn to_pubkey(&self) -> Pubkey {
    //         Pubkey::from(self.to_bytes())
    //     }
    // }

    // impl PubkeyExt for Address {
    //     fn to_address(&self) -> Address {
    //         *self
    //     }
    //     fn to_pubkey(&self) -> Pubkey {
    //         Pubkey::from(self.to_bytes())
    //     }
    // }

    use gated_vault_transfer_hook::ID as PROGRAM_ID;
    const TRANSFER_HOOK_PROGRAM_ID: Pubkey = transfer_hook::ID;

    pub struct ReusableData {
        ata_program: Pubkey,
        token_program: Pubkey,
        system_program: Pubkey,
        vault_ata: Pubkey,
        vault_state: Pubkey,
        mint: Keypair,
        admin: Keypair,
    }

    // pub fn pubkey_to_address(pubkey: &Pubkey) -> Address {
    //     Address::from(pubkey.to_bytes())
    // }

    pub fn setup() -> (LiteSVM, ReusableData) {
        let mut svm = LiteSVM::new();
        let admin = Keypair::new();

        svm.airdrop(&admin.pubkey(), 10 * LAMPORTS_PER_SOL)
            .expect("Failed to airdrop SOL to payer");

        println!("path is here: {}", env!("CARGO_MANIFEST_DIR"));

        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

        // Go up ONE level from /tests to get to the workspace root
        let workspace_root = manifest_dir
            .parent()
            .expect("Failed to find workspace root");

        let so_path = workspace_root.join("target/deploy/gated_vault_transfer_hook.so");

        let transfer_hook_so_path = workspace_root.join("target/deploy/transfer_hook.so");

        // Load program SO file
        // let so_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        //     .join("../../target/deploy/gated_vault_transfer_hook.so");

        // let transfer_hook_so_path =
        //     PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../target/deploy/transfer_hook.so");

        let program_data = std::fs::read(so_path).expect("Failed to read vault program SO file");
        let hook_program_data =
            std::fs::read(transfer_hook_so_path).expect("Failed to read hook program SO file");

        let program_id = PROGRAM_ID;
        let hook_program_id = TRANSFER_HOOK_PROGRAM_ID;
        let spl_program_id = spl_token_2022::ID;
        let ata_program_id = spl_associated_token_account::ID;
        let system_program_id = SYSTEM_PROGRAM_ID;

        let mint = Keypair::new();

        svm.add_program(hook_program_id, &hook_program_data)
            .expect("failed to add hook program");

        svm.add_program(program_id, &program_data)
            .expect("failed to add vault program");

        let vault_state =
            Pubkey::find_program_address(&[&mint.pubkey().to_bytes(), b"vault"], &program_id).0;

        let vault_ata = associated_token::get_associated_token_address_with_program_id(
            &vault_state,
            &mint.pubkey(),
            &spl_program_id,
        );

        let exported_state = ReusableData {
            admin: admin,
            ata_program: ata_program_id,
            system_program: system_program_id,
            token_program: spl_program_id,
            mint,
            vault_ata: vault_ata,
            vault_state,
        };

        (svm, exported_state)
    }

    #[test]
    pub fn test_full_deposit_workflow() {
        let (mut svm, reusable_data) = setup();
        msg!("here's the hook id: {}", TRANSFER_HOOK_PROGRAM_ID);
        msg!(" 🔥 [1] create vault");
        // 🔥 [1] create vault

        let create_vault_ix = Instruction {
            program_id: PROGRAM_ID,
            accounts: gated_vault_transfer_hook::accounts::VaultOperation {
                associated_token_program: reusable_data.ata_program,
                hook_program_id: TRANSFER_HOOK_PROGRAM_ID,
                mint: reusable_data.mint.pubkey(),
                owner: reusable_data.admin.pubkey(),
                system_program: reusable_data.system_program,
                token_program: reusable_data.token_program,
                vault_ata: reusable_data.vault_ata,
                vault_state: reusable_data.vault_state,
            }
            .to_account_metas(None),
            data: gated_vault_transfer_hook::instruction::CreateVault {}.data(),
        };

        let recent_blockhash = svm.latest_blockhash();

        let transaction1 = Transaction::new_signed_with_payer(
            &[create_vault_ix],
            Some(&reusable_data.admin.pubkey()),
            &[&reusable_data.admin, &reusable_data.mint],
            recent_blockhash,
        );

        svm.send_transaction(transaction1).unwrap();

        let new_state_of_vault = svm.get_account(&reusable_data.vault_state).unwrap();
        let fetched_vault_state = gated_vault_transfer_hook::state::Vault::try_deserialize(
            &mut new_state_of_vault.data.as_ref(),
        )
        .unwrap();

        assert_eq!(
            fetched_vault_state.mint.key(),
            reusable_data.mint.pubkey(),
            "Account wasn't set correctly in Vault"
        );

        assert_eq!(
            fetched_vault_state.owner.key(),
            reusable_data.admin.pubkey(),
            "Account wasn't set correctly in Vault"
        );

        let mint_account = svm.get_account(&reusable_data.mint.pubkey()).unwrap();
        msg!("MINT Data Length: {}", mint_account.data.len());

        // 🔥🔥 [2] initialize transfer hook

        let extra_account_meta_list = Pubkey::find_program_address(
            &[
                b"extra-account-metas",
                &reusable_data.mint.pubkey().as_ref(),
            ],
            &TRANSFER_HOOK_PROGRAM_ID,
        )
        .0;

        let initialize_transfer_hook_ix = Instruction {
            program_id: TRANSFER_HOOK_PROGRAM_ID,
            accounts: transfer_hook::accounts::InitializeExtraAccountMetaList {
                extra_account_meta_list,
                mint: reusable_data.mint.pubkey(),
                payer: reusable_data.admin.pubkey(),
                system_program: reusable_data.system_program.key(),
            }
            .to_account_metas(None),
            data: transfer_hook::instruction::InitializeTransferHook {}.data(),
        };

        let recent_blockhash = svm.latest_blockhash();

        let transaction2 = Transaction::new_signed_with_payer(
            &[initialize_transfer_hook_ix],
            Some(&reusable_data.admin.pubkey()),
            &[&reusable_data.admin],
            recent_blockhash,
        );

        svm.send_transaction(transaction2).unwrap();

        let new_state_of_metalist = svm.get_account(&extra_account_meta_list).unwrap();
        // let fetched_metalist_state =
        //     crate::AccountInfo::try_borrow_data(&new_state_of_metalist).unwrap();

        msg!("MetaList state: {:?}", new_state_of_metalist.data);

        // 🔥🔥🔥🔥 [3] add to whitelist
        let whitelist = Pubkey::find_program_address(&[b"whitelist"], &PROGRAM_ID.key()).0;

        let new_user = Keypair::new();

        svm.airdrop(&new_user.pubkey(), 10 * LAMPORTS_PER_SOL)
            .expect("Failed to airdrop SOL to payer");

        let new_user_ata = associated_token::get_associated_token_address_with_program_id(
            &new_user.pubkey(),
            &reusable_data.mint.pubkey(),
            &reusable_data.token_program.key(),
        );

        let add_to_whitelist_ix = Instruction {
            program_id: PROGRAM_ID,
            accounts: gated_vault_transfer_hook::accounts::WhitelistOperations {
                whitelist: whitelist.key(),
                vault: reusable_data.vault_state.key(),
                admin: reusable_data.admin.pubkey(),
                system_program: reusable_data.system_program.key(),
            }
            .to_account_metas(None),
            data: gated_vault_transfer_hook::instruction::AddToWhitelist {
                address: new_user_ata.key(),
                _mint: reusable_data.mint.pubkey(),
            }
            .data(),
        };

        let recent_blockhash = svm.latest_blockhash();

        let transaction3 = Transaction::new_signed_with_payer(
            &[add_to_whitelist_ix],
            Some(&reusable_data.admin.pubkey()),
            &[&reusable_data.admin],
            recent_blockhash,
        );

        svm.send_transaction(transaction3).unwrap();

        let whitelist_account_info = svm.get_account(&whitelist).unwrap();
        let mut whitelist_data_slice: &[u8] = whitelist_account_info.data.as_ref();

        let fetched_whitelist_state =
            gated_vault_transfer_hook::state::Whitelist::try_deserialize(&mut whitelist_data_slice)
                .expect("Failed to deserialize Whitelist account data. Check initialization.");

        // 4. Use the deserialized struct to check the address.
        let contains_address = fetched_whitelist_state.contains_address(&new_user_ata.key());

        assert!(
            contains_address,
            "You were not successfully added to the whitelist"
        );
        msg!("contains you: {}", contains_address);

        msg!("🔥🔥🔥 [4] mint token to self");
        // 🔥🔥🔥 [4] mint token to self

        // let new_user = Keypair::new();
        let admin_ata = associated_token::get_associated_token_address_with_program_id(
            &reusable_data.admin.pubkey(),
            &reusable_data.mint.pubkey(),
            &reusable_data.token_program.key(),
        );

        //  svm.airdrop(&admin.pubkey(), 10 * LAMPORTS_PER_SOL)
        //     .expect("Failed to airdrop SOL to payer");

        let mint_token_ix = Instruction {
            program_id: PROGRAM_ID,
            accounts: gated_vault_transfer_hook::accounts::TokenFactory {
                extra_account_meta_list,
                mint: reusable_data.mint.pubkey(),
                associated_token_program: reusable_data.ata_program.key(),
                blocklist: whitelist,
                hook_program_id: TRANSFER_HOOK_PROGRAM_ID.key(),
                source_token_account: admin_ata.key(),
                system_program: reusable_data.system_program.key(),
                token_program: reusable_data.token_program.key(),
                user: reusable_data.admin.pubkey(),
            }
            .to_account_metas(None),
            data: gated_vault_transfer_hook::instruction::MintToken {
                amount: 10_000,
                decimals: 9,
            }
            .data(),
        };

        let recent_blockhash = svm.latest_blockhash();

        let transaction4 = Transaction::new_signed_with_payer(
            &[mint_token_ix],
            Some(&reusable_data.admin.pubkey()),
            &[&reusable_data.admin],
            recent_blockhash,
        );

        svm.send_transaction(transaction4).unwrap();

        // 1. Fetch the raw account data
        let new_state_of_admin_ata = svm.get_account(&admin_ata).unwrap();

        // 2. Get the data as a mutable slice (&[u8] or &mut [u8])
        let account_data_slice: &[u8] = new_state_of_admin_ata.data.as_ref();

        const AMOUNT_OFFSET: usize = 64; // The fixed position of the u64 'amount' field
        const U64_SIZE: usize = 8;

        if account_data_slice.len() < AMOUNT_OFFSET + U64_SIZE {
            panic!(
                "FATAL: Account data too short to contain token balance. Length: {}",
                account_data_slice.len()
            );
        }

        // Read 8 bytes starting at offset 64
        let amount_bytes: [u8; U64_SIZE] = account_data_slice
            [AMOUNT_OFFSET..AMOUNT_OFFSET + U64_SIZE]
            .try_into()
            .expect("Slice to array conversion failed");

        // Convert the bytes to a u64 (Solana uses little-endian)
        let token_balance = u64::from_le_bytes(amount_bytes);

        // --- Verification ---

        // You are now successfully reading the balance directly.
        msg!("Admin ATA token balance (POD Read): {}", token_balance);

        // Assert that the mint was successful (10,000 tokens)
        assert_eq!(
            token_balance, 10_000,
            "Token balance after minting is incorrect."
        );

        msg!("🔥🔥🔥🔥🔥 [5] deposit");
        // 🔥🔥🔥🔥🔥 [5] deposit

        let create_ata_ix =
            spl_associated_token_account::instruction::create_associated_token_account(
                &new_user.pubkey(),
                &new_user.pubkey(),
                &reusable_data.mint.pubkey(),
                &reusable_data.token_program.key(),
            );

        let recent_blockhash_ata = svm.latest_blockhash();

        let transaction_create_ata = Transaction::new_signed_with_payer(
            &[create_ata_ix],
            Some(&new_user.pubkey()),
            &[&new_user], // Must be signed by the payer
            recent_blockhash_ata,
        );

        svm.send_transaction(transaction_create_ata).unwrap();

        let mint_to_ix = spl_token_2022::instruction::mint_to_checked(
            &reusable_data.token_program.key(),
            &reusable_data.mint.pubkey(),
            &new_user_ata,
            &reusable_data.admin.pubkey(),
            &[],    // Signer Pubkeys (not needed if authority signs the tx)
            20_000, // Amount
            9,      // Decimals
        )
        .unwrap();

        let recent_blockhash = svm.latest_blockhash();

        let transaction_mint = Transaction::new_signed_with_payer(
            &[mint_to_ix],
            Some(&reusable_data.admin.pubkey()),
            &[&reusable_data.admin],
            recent_blockhash,
        );
        svm.send_transaction(transaction_mint).unwrap();

        // confirm that the mint went through successfully
        let new_state_of_user_ata = svm.get_account(&new_user_ata).unwrap();
        let user_ata_slice: &[u8] = new_state_of_user_ata.data.as_ref();

        let amount_bytes: [u8; U64_SIZE] = user_ata_slice[AMOUNT_OFFSET..AMOUNT_OFFSET + U64_SIZE]
            .try_into()
            .expect("Slice to array conversion failed");

        // Convert the bytes to a u64 (Solana uses little-endian)
        let new_user_token_balance = u64::from_le_bytes(amount_bytes);

        msg!(
            "User ATA token balance (POD Read): {}",
            new_user_token_balance
        );

        assert_eq!(
            new_user_token_balance, 20_000,
            "Token balance after minting is incorrect."
        );

        let deposit_to_vault_ix = Instruction {
            program_id: PROGRAM_ID,
            accounts: gated_vault_transfer_hook::accounts::DepositWithdraw {
                associated_token_program: reusable_data.ata_program.key(),
                hook_program_id: TRANSFER_HOOK_PROGRAM_ID.key(),
                mint: reusable_data.mint.pubkey(),
                owner: reusable_data.admin.pubkey(),
                sender: new_user.pubkey(),
                system_program: reusable_data.system_program.key(),
                token_program: reusable_data.token_program.key(),
                user_ata: new_user_ata.key(),
                vault_ata: reusable_data.vault_ata.key(),
                vault_state: reusable_data.vault_state.key(),
                extra_account_meta_list: extra_account_meta_list.key(),
                whitelist: whitelist.key(),
            }
            .to_account_metas(None),
            data: gated_vault_transfer_hook::instruction::Deposit { amount: 100 }.data(),
        };

        // deposit_to_vault_ix.accounts.push(AccountMeta {
        //     pubkey: whitelist.key(),
        //     is_signer: false,
        //     is_writable: false,
        // });

        let transaction5 = Transaction::new_signed_with_payer(
            &[deposit_to_vault_ix],
            Some(&new_user.pubkey()),
            &[&new_user],
            recent_blockhash,
        );

        svm.send_transaction(transaction5).unwrap();

        msg!("🔥🔥🔥🔥🔥 [6] remove from whitelist");
        // 🔥🔥🔥🔥🔥 [6] remove from whitelist
        // let remove_from_whitelist_ix = Instruction {
        //     program_id: PROGRAM_ID,
        //     accounts: crate::accounts::WhitelistOperations {
        //         whitelist: whitelist.key(),
        //         vault: reusable_data.vault_state.key(),
        //         admin: reusable_data.admin.pubkey(),
        //         system_program: reusable_data.system_program.key(),
        //     }
        //     .to_account_metas(None),
        //     data: crate::instruction::RemoveFromWhitelist {
        //         address: new_user_ata.key(),
        //         mint: reusable_data.mint.pubkey(),
        //     }
        //     .data(),
        // };

        // let recent_blockhash = svm.latest_blockhash();

        // let transaction3 = Transaction::new_signed_with_payer(
        //     &[remove_from_whitelist_ix],
        //     Some(&reusable_data.admin.pubkey()),
        //     &[&reusable_data.admin],
        //     recent_blockhash,
        // );

        // svm.send_transaction(transaction3).unwrap();

        msg!("🔥🔥🔥🔥🔥 [7] withdraw");
        // 🔥🔥🔥🔥🔥 [7] withdraw
        let withdraw_from_vault_ix = Instruction {
            program_id: PROGRAM_ID,
            accounts: gated_vault_transfer_hook::accounts::DepositWithdraw {
                associated_token_program: reusable_data.ata_program.key(),
                hook_program_id: TRANSFER_HOOK_PROGRAM_ID.key(),
                mint: reusable_data.mint.pubkey(),
                owner: reusable_data.admin.pubkey(),
                sender: new_user.pubkey(),
                system_program: reusable_data.system_program.key(),
                token_program: reusable_data.token_program.key(),
                user_ata: new_user_ata.key(),
                vault_ata: reusable_data.vault_ata.key(),
                vault_state: reusable_data.vault_state.key(),
                extra_account_meta_list: extra_account_meta_list.key(),
                whitelist: whitelist.key(),
            }
            .to_account_metas(None),
            data: gated_vault_transfer_hook::instruction::Withdraw { amount: 100 }.data(),
        };

        let recent_blockhash = svm.latest_blockhash();

        let transaction7 = Transaction::new_signed_with_payer(
            &[withdraw_from_vault_ix],
            Some(&new_user.pubkey()),
            &[&new_user],
            recent_blockhash,
        );

        svm.send_transaction(transaction7).unwrap();
    }
}
