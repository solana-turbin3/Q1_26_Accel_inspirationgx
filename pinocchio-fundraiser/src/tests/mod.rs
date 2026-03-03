pub mod create_fundraiser;
pub mod donate;

#[cfg(test)]
mod tests {

    use litesvm::LiteSVM;
    use litesvm_token::{
        spl_token::{self},
        CreateAssociatedTokenAccount, CreateMint,
    };

    use solana_keypair::Keypair;

    use solana_native_token::LAMPORTS_PER_SOL;
    use solana_pubkey::Pubkey;
    use solana_sdk_ids::system_program;
    use solana_signer::Signer;

    use crate::tests::{
        create_fundraiser::create_fundraiser::create_fundraiser_function,
        donate::donate::donate_function,
    };

    const PROGRAM_ID: Pubkey = crate::ID;
    const TOKEN_PROGRAM_ID: Pubkey = spl_token::ID;
    const ASSOCIATED_TOKEN_PROGRAM_ID: &str = "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";

    pub struct ReusableState {
        pub mint: Pubkey,
        pub maker_ata: Pubkey,
        pub vault: Pubkey,
        pub ata_program: Pubkey,
        pub token_program: Pubkey,
        pub system_program: Pubkey,
        pub fundraiser: (Pubkey, u8),
        pub maker: Keypair,
        pub user: Option<Keypair>,
        pub user_ata: Option<Pubkey>,
        pub user_pda: Option<(Pubkey, u8)>,
    }

    pub fn program_id() -> Pubkey {
        Pubkey::from(crate::ID)
    }

    fn setup() -> (LiteSVM, ReusableState) {
        let mut svm = LiteSVM::new();
        let payer = Keypair::new();

        svm.airdrop(&payer.pubkey(), 10 * LAMPORTS_PER_SOL)
            .expect("Airdrop failed");

        println!("The path is!! {}", env!("CARGO_MANIFEST_DIR"));

        let bytes = include_bytes!("../../target/deploy/fundraiser.so");

        // Load program SO file
        // println!("The path is!! {}", env!("CARGO_MANIFEST_DIR"));
        // let so_path = PathBuf::from("/Users/andrecorreia/Documents/Solana/pinocchio-escrow-2025/escrow/target/sbf-solana-solana/release/escrow.so");

        // let program_data = std::fs::read(so_path).expect("Failed to read program SO file");

        // let maker_keypair = Keypair::new();
        // let maker_pubkey = maker_keypair.pubkey();

        let maker_pubukey = payer.pubkey();

        let fundraiser_seed = [b"fundraiser", maker_pubukey.as_ref()];
        let fundraiser = Pubkey::find_program_address(&fundraiser_seed, &crate::ID);

        println!("Fundraiser: {}", &fundraiser.0.to_string());

        let mint = CreateMint::new(&mut svm, &payer)
            .decimals(6)
            .authority(&payer.pubkey())
            .send()
            .unwrap();
        println!("Mint A: {}", mint);

        let maker_ata = CreateAssociatedTokenAccount::new(&mut svm, &payer, &mint)
            .owner(&payer.pubkey())
            .send()
            .unwrap();
        println!("Maker ATA A: {}\n", maker_ata);

        let vault = spl_associated_token_account::get_associated_token_address(
            &fundraiser.0, // owner will be the fundraiser PDA
            &mint,         // mint
        );
        println!("Vault PDA: {}\n", vault);

        svm.add_program(program_id(), bytes)
            .expect("Failed to add program");

        let reusable_state = ReusableState {
            ata_program: ASSOCIATED_TOKEN_PROGRAM_ID.parse::<Pubkey>().unwrap(),
            fundraiser,
            maker: payer,
            maker_ata,
            mint,
            system_program: system_program::ID,
            token_program: TOKEN_PROGRAM_ID,
            user: None,
            user_ata: None,
            user_pda: None,
            vault,
        };

        (svm, reusable_state)
    }

    #[test]
    pub fn test_create_fundraiser_instruction() {
        let (mut svm, reusable_state) = setup();
        create_fundraiser_function(&mut svm, &reusable_state).unwrap();
    }

    #[test]
    pub fn test_donate_instruction() {
        let (mut svm, reusable_state) = setup();
        create_fundraiser_function(&mut svm, &reusable_state).unwrap();
        donate_function(&mut svm, &reusable_state).unwrap();
    }
}
