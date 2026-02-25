# Anchor Escrow with LiteSVM

This example demonstrates how to build a trustless token escrow program on Solana using the Anchor framework, and how to test it entirely in Rust using [LiteSVM](https://github.com/LiteSVM/litesvm) — a fast, in-process Solana VM for unit testing.

In this escrow, a **maker** deposits tokens of one mint (Mint A) into a program-controlled vault and specifies how many tokens of another mint (Mint B) they want in return. A **taker** can fulfill the trade by sending the requested Mint B tokens to the maker, after which they receive the Mint A tokens from the vault. The maker can also cancel the escrow at any time by reclaiming their deposit. A **lock period** is enforced so the taker cannot fulfill the trade before a specified timestamp.

---

## Let's walk through the architecture:

For this program, we will have 1 main state account:

- An Escrow account

An Escrow account consists of:

```rust
#[account]
#[derive(InitSpace, Debug)]
pub struct Escrow {
    pub seed: u64,
    pub maker: Pubkey,
    pub mint_a: Pubkey,
    pub mint_b: Pubkey,
    pub receive: u64,
    pub lock_period: i64,
    pub bump: u8,
}
```

### In this state account, we will store:

- seed: A user-provided 64-bit unsigned integer used as part of the PDA derivation, allowing the same maker to open multiple escrows simultaneously.
- maker: The public key of the wallet that created and funded the escrow.
- mint_a: The mint of the token the maker is depositing (offered token).
- mint_b: The mint of the token the maker wants to receive in exchange.
- receive: The amount of Mint B tokens the maker expects from the taker.
- lock_period: A Unix timestamp before which the taker cannot fulfill the trade.
- bump: The bump seed used to derive the escrow PDA.

---

### The maker will be able to open a new escrow. For that, we create the following context:

```rust
#[derive(Accounts)]
#[instruction(seed: u64)]
pub struct Make<'info> {
    #[account(mut)]
    pub maker: Signer<'info>,
    pub mint_a: InterfaceAccount<'info, Mint>,
    pub mint_b: InterfaceAccount<'info, Mint>,
    #[account(
        mut,
        associated_token::mint = mint_a,
        associated_token::authority = maker,
    )]
    pub maker_ata_a: InterfaceAccount<'info, TokenAccount>,
    #[account(
        init,
        payer = maker,
        seeds = [b"escrow", maker.key().as_ref(), seed.to_le_bytes().as_ref()],
        bump,
        space = 8 + Escrow::INIT_SPACE,
    )]
    pub escrow: Account<'info, Escrow>,
    #[account(
        init,
        payer = maker,
        associated_token::mint = mint_a,
        associated_token::authority = escrow,
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}
```

Let's have a closer look at the accounts that we are passing in this context:

- maker: The wallet opening the escrow. He will be a signer of the transaction, and we mark his account as mutable as we will be deducting lamports and tokens from this account.

- mint_a: The mint of the token the maker is depositing.

- mint_b: The mint of the token the maker wants to receive. It is only referenced here for storage in the escrow state.

- maker_ata_a: The maker's associated token account for Mint A, from which tokens will be transferred into the vault.

- escrow: The PDA state account initialized during this instruction. Its address is derived from the bytes `"escrow"`, the maker's public key, and the provided seed, giving each escrow a unique address.

- vault: An associated token account for Mint A whose authority is the escrow PDA. This account holds the deposited tokens on behalf of the escrow.

- associated_token_program, token_program, system_program: Programs required for account creation and token operations.

### We then implement some functionality for our Make context:

```rust
impl<'info> Make<'info> {
    pub fn init_escrow(
        &mut self,
        seed: u64,
        receive: u64,
        lock_period: i64,
        bumps: &MakeBumps,
    ) -> Result<()> {
        self.escrow.set_inner(Escrow {
            seed,
            maker: self.maker.key(),
            mint_a: self.mint_a.key(),
            mint_b: self.mint_b.key(),
            receive,
            lock_period,
            bump: bumps.escrow,
        });
        Ok(())
    }

    pub fn deposit(&mut self, deposit: u64) -> Result<()> {
        let cpi_accounts = TransferChecked {
            from: self.maker_ata_a.to_account_info(),
            to: self.vault.to_account_info(),
            authority: self.maker.to_account_info(),
            mint: self.mint_a.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(self.token_program.to_account_info(), cpi_accounts);
        transfer_checked(cpi_ctx, deposit, self.mint_a.decimals)?;
        Ok(())
    }
}
```

In `init_escrow`, we populate all fields of the Escrow state account. In `deposit`, we perform a `transfer_checked` CPI to move the specified amount of Mint A tokens from the maker's ATA into the vault. Both are called atomically from the `make` entrypoint.

---

### The taker will be able to fulfill the escrow trade. For that, we create the following context:

```rust
#[derive(Accounts)]
pub struct Take<'info> {
    #[account(mut)]
    pub taker: Signer<'info>,
    #[account(mut)]
    pub maker: SystemAccount<'info>,
    pub mint_a: InterfaceAccount<'info, Mint>,
    pub mint_b: InterfaceAccount<'info, Mint>,
    #[account(
        init_if_needed,
        payer = taker,
        associated_token::mint = mint_a,
        associated_token::authority = taker,
    )]
    pub taker_ata_a: InterfaceAccount<'info, TokenAccount>,
    #[account(
        mut,
        associated_token::mint = mint_b,
        associated_token::authority = taker,
    )]
    pub taker_ata_b: InterfaceAccount<'info, TokenAccount>,
    #[account(
        init_if_needed,
        payer = taker,
        associated_token::mint = mint_b,
        associated_token::authority = maker,
    )]
    pub maker_ata_b: InterfaceAccount<'info, TokenAccount>,
    #[account(
        mut,
        close = maker,
        has_one = maker,
        has_one = mint_a,
        has_one = mint_b,
        seeds = [b"escrow", maker.key().as_ref(), escrow.seed.to_le_bytes().as_ref()],
        bump = escrow.bump,
    )]
    pub escrow: Account<'info, Escrow>,
    #[account(
        mut,
        associated_token::mint = mint_a,
        associated_token::authority = escrow,
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}
```

In this context, we are passing all the accounts needed for the taker to fulfill the escrow:

- taker: The wallet fulfilling the trade. He will be a signer of the transaction, and we mark his account as mutable as he may pay for account creation.

- maker: The original maker's wallet. Marked mutable as it will receive the rent from the closed escrow and vault accounts, as well as the Mint B tokens.

- mint_a, mint_b: The two token mints involved in the swap. They are cross-checked against the values stored in the escrow state.

- taker_ata_a: The taker's ATA for Mint A, created if needed. This is where the vault tokens will be sent.

- taker_ata_b: The taker's ATA for Mint B, from which the `receive` amount is transferred to the maker.

- maker_ata_b: The maker's ATA for Mint B, created if needed. This is where the taker's Mint B payment lands.

- escrow: The escrow state account. Constraints verify it belongs to the correct maker and mints. The `close = maker` constraint causes the account rent to be refunded to the maker on successful settlement.

- vault: The escrow-controlled token account holding the Mint A deposit. Closed after the swap.

### We then implement some functionality for our Take context:

```rust
impl<'info> Take<'info> {
    pub fn deposit(&mut self) -> Result<()> {
        let clock = Clock::get()?;
        require!(
            self.escrow.lock_period <= clock.unix_timestamp,
            EscrowError::LockPeriodNotElapsed
        );

        let cpi_accounts = TransferChecked {
            from: self.taker_ata_b.to_account_info(),
            to: self.maker_ata_b.to_account_info(),
            authority: self.taker.to_account_info(),
            mint: self.mint_b.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(self.token_program.to_account_info(), cpi_accounts);
        transfer_checked(cpi_ctx, self.escrow.receive, self.mint_b.decimals)
    }

    pub fn withdraw_and_close_vault(&mut self) -> Result<()> {
        let signer_seeds: [&[&[u8]]; 1] = [&[
            b"escrow",
            self.maker.key.as_ref(),
            &self.escrow.seed.to_le_bytes()[..],
            &[self.escrow.bump],
        ]];

        // Transfer Mint A from vault to taker
        let cpi_accounts = TransferChecked { ... };
        transfer_checked(cpi_context, self.vault.amount, self.mint_a.decimals)?;

        // Close the vault account
        let cpi_accounts = CloseAccount { ... };
        close_account(cpi_context)
    }
}
```

In `deposit`, we first enforce the lock period by comparing the current on-chain clock against the stored `lock_period` timestamp. If the lock has not elapsed, the transaction fails with `EscrowError::LockPeriodNotElapsed`. If the lock has passed, we perform a `transfer_checked` CPI to move the exact `receive` amount of Mint B from the taker to the maker.

In `withdraw_and_close_vault`, we use the escrow PDA's signer seeds to sign on behalf of the vault authority, transfer all Mint A tokens from the vault to the taker's ATA, and then close the vault account, refunding its rent to the maker.

---

### The maker will be able to cancel and reclaim their deposit. For that, we create the following context:

```rust
#[derive(Accounts)]
pub struct Refund<'info> {
    #[account(mut)]
    maker: Signer<'info>,
    mint_a: InterfaceAccount<'info, Mint>,
    #[account(
        mut,
        associated_token::mint = mint_a,
        associated_token::authority = maker,
    )]
    maker_ata_a: InterfaceAccount<'info, TokenAccount>,
    #[account(
        mut,
        close = maker,
        has_one = mint_a,
        has_one = maker,
        seeds = [b"escrow", maker.key().as_ref(), escrow.seed.to_le_bytes().as_ref()],
        bump = escrow.bump,
    )]
    pub escrow: Account<'info, Escrow>,
    #[account(
        mut,
        associated_token::mint = mint_a,
        associated_token::authority = escrow,
    )]
    vault: InterfaceAccount<'info, TokenAccount>,
    token_program: Interface<'info, TokenInterface>,
    system_program: Program<'info, System>,
}
```

In this context, we are passing all the accounts needed for the maker to cancel the escrow:

- maker: The original creator of the escrow, required as a signer to prevent unauthorized cancellations.

- mint_a: The mint of the deposited tokens, verified via `has_one` against the escrow state.

- maker_ata_a: The maker's ATA for Mint A, which will receive the refunded tokens.

- escrow: The escrow state account. The `close = maker` constraint returns the rent to the maker on close.

- vault: The token account holding the deposited Mint A tokens to be returned.

### We then implement some functionality for our Refund context:

```rust
impl<'info> Refund<'info> {
    pub fn refund_and_close_vault(&mut self) -> Result<()> {
        let signer_seeds: [&[&[u8]]; 1] = [&[
            b"escrow",
            self.maker.key.as_ref(),
            &self.escrow.seed.to_le_bytes()[..],
            &[self.escrow.bump]
        ]];

        // Transfer all Mint A tokens from vault back to maker
        transfer_checked(cpi_context, self.vault.amount, self.mint_a.decimals)?;

        // Close the vault account, refunding rent to the maker
        close_account(cpi_context)?;

        Ok(())
    }
}
```

In `refund_and_close_vault`, we use the escrow PDA's signer seeds to authorize the transfer back from the vault to the maker's ATA, then close the vault account and return its rent to the maker. Anchor automatically closes the escrow state account and returns its rent via the `close = maker` constraint.

---

## Testing with LiteSVM

This project uses [LiteSVM](https://github.com/LiteSVM/litesvm) for fast, in-process unit testing in Rust — no local validator or TypeScript test harness required.

LiteSVM runs a lightweight Solana VM in-process, making tests start instantly and execute at native speed. Tests are written in Rust inside `src/tests/mod.rs` and load the compiled `.so` file from the `target/deploy/` directory.

The test suite covers:

- **`test_make`** — mints Mint A tokens to the maker, opens an escrow with a 2-day lock period, and asserts that the vault holds the correct amount and is owned by the escrow PDA.

- **`test_take`** — sets up a full make-and-take scenario, advances the simulated clock past the lock period using `program.set_sysvar::<Clock>()`, and verifies the entire swap completes successfully.

---

## Flow of Actions

Here is the step-by-step flow to build and test the escrow program:

### 1. Build the program

```bash
anchor build
```

This compiles the program and outputs the `.so` file to `target/deploy/anchor_escrow.so`, which the LiteSVM tests load at runtime.

### 2. Run the tests

```bash
cargo test -- --nocapture
```

The `--nocapture` flag allows `msg!` print statements to appear in the terminal output. Tests run entirely in-process via LiteSVM — no `solana-test-validator` is needed.

### 3. Deploy to a network (optional)

```bash
anchor deploy
```

Deploys the program to the cluster configured in `Anchor.toml` (defaults to localnet).

---

This Anchor Escrow demonstrates a trustless token swap pattern on Solana, featuring time-locked settlement enforced on-chain and fully in-Rust unit tests powered by LiteSVM for fast, deterministic test execution.
