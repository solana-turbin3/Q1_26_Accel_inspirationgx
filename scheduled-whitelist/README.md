# Scheduled Whitelist Transfer Hook

This example demonstrates how to combine the SPL Token 2022 Transfer Hook interface with [TukTuk](https://github.com/helium/tuktuk) — Helium's decentralized task queue system — to enforce whitelist restrictions on token transfers and automate their removal on-chain.

In this example, each address that is permitted to transfer tokens has its own per-user whitelist PDA, and TukTuk is used to schedule automatic whitelist removal so that access can be revoked without requiring a manual follow-up transaction.

---

## Let's walk through the architecture:

For this program, we will have 1 main state account:

- A Whitelist account

A Whitelist account consists of:

```rust
#[account]
#[derive(InitSpace)]
pub struct Whitelist {
    pub address: Pubkey,
    pub is_whitelisted: bool,
    pub bump: u8,
}
```

### In this state account, we will store:

- address: The public key of the wallet this PDA belongs to and represents in the whitelist.
- is_whitelisted: A boolean flag indicating whether this address is currently permitted to transfer tokens.
- bump: The bump seed used to derive this PDA.

Unlike a single shared whitelist, every wallet that participates in this system has its own dedicated Whitelist PDA. The PDA is derived from the byte literal `"whitelist"` and the wallet's public key, making each account uniquely bound to a single address. The fixed-size layout has a total space of 42 bytes (8-byte discriminator + 32-byte Pubkey + 1-byte bool + 1-byte bump).

---

### The admin will be able to create a new per-user Whitelist account. For that, we create the following context:

```rust
#[derive(Accounts)]
#[instruction(user_wallet: Pubkey)]
pub struct InitializeWhitelist<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(
        init,
        payer = admin,
        space = Whitelist::DISCRIMINATOR.len() + Whitelist::INIT_SPACE,
        seeds = [b"whitelist", user_wallet.key().as_ref()],
        bump
    )]
    pub whitelist: Account<'info, Whitelist>,
    pub system_program: Program<'info, System>,
}
```

Let's have a closer look at the accounts that we are passing in this context:

- admin: Will be the person creating the whitelist account. He will be a signer of the transaction, and we mark his account as mutable as we will be deducting lamports from this account.

- whitelist: Will be the per-user state account that we will initialize and the admin will be paying for the initialization of the account. We derive this Whitelist PDA from the byte representation of the word "whitelist" and the `user_wallet` public key passed as an instruction argument.

- system_program: Program responsible for the initialization of any new account.

### We then implement some functionality for our InitializeWhitelist context:

```rust
impl<'info> InitializeWhitelist<'info> {
    pub fn initialize_whitelist(&mut self, bumps: InitializeWhitelistBumps, user_wallet: Pubkey) -> Result<()> {
        self.whitelist.set_inner(Whitelist {
            address: user_wallet,
            bump: bumps.whitelist,
            is_whitelisted: false
        });
        Ok(())
    }
}
```

In here, we set the initial data of our Whitelist account: the `address` field is set to the provided `user_wallet` public key, the `is_whitelisted` flag starts as `false`, and we store the bump seed for future PDA derivation.

---

### The admin will be able to manage whitelist operations (add/remove/close) for a specific user:

```rust
#[derive(Accounts)]
pub struct WhitelistOperations<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(
        mut @WhitelistError::NotInitialized,
        seeds = [b"whitelist", user_wallet.key().as_ref()],
        bump,
    )]
    pub whitelist: Account<'info, Whitelist>,
    #[account(
        mut,
        constraint = user_wallet.key() == whitelist.address.key() @WhitelistError::WrongAccount
    )]
    pub user_wallet: SystemAccount<'info>,
    pub system_program: Program<'info, System>,
}
```

In this context, we are passing all the accounts needed to manage the whitelist for a specific user:

- admin: The address of the platform admin. He will be a signer of the transaction, and we mark his account as mutable as he may need to cover any lamports involved in account operations.

- whitelist: The per-user state account that we will modify. We derive the Whitelist PDA from the byte representation of the word "whitelist" and the `user_wallet` public key. The `@WhitelistError::NotInitialized` constraint ensures the account exists before any operation is attempted.

- user_wallet: The system account whose whitelist entry we are managing. The `constraint` enforces that the `user_wallet` passed in matches the `address` stored inside the Whitelist PDA, preventing operations on the wrong account.

- system_program: Program responsible for account operations including closing accounts.

### We then implement some functionality for our WhitelistOperations context:

```rust
impl<'info> WhitelistOperations<'info> {
    pub fn add_to_whitelist(&mut self, user_wallet: Pubkey) -> Result<()> {
        if self.whitelist.is_whitelisted {
            panic!("User is already whitelisted");
        }
        self.whitelist.is_whitelisted = true;
        Ok(())
    }

    pub fn remove_from_whitelist(&mut self, user_wallet: Pubkey) -> Result<()> {
        if !self.whitelist.is_whitelisted {
            panic!("User is not whitelisted but account exists");
        }
        self.whitelist.is_whitelisted = false;
        Ok(())
    }

    pub fn close_whitelist(&mut self) -> Result<()> {
        self.whitelist.close(self.user_wallet.to_account_info())?;
        Ok(())
    }
}
```

In here, we implement three operations on the per-user Whitelist PDA:

- `add_to_whitelist` flips `is_whitelisted` to `true`, guarding against double-adding.
- `remove_from_whitelist` flips `is_whitelisted` to `false`, guarding against removing an address that was never active.
- `close_whitelist` closes the PDA entirely using Anchor's built-in `close` method, refunding the rent lamports back to the `user_wallet`.

---

### The system will need to initialize extra account metadata for the transfer hook:

```rust
#[derive(Accounts)]
pub struct InitializeExtraAccountMetaList<'info> {
    #[account(mut)]
    payer: Signer<'info>,
    #[account(
        init,
        seeds = [b"extra-account-metas", mint.key().as_ref()],
        bump,
        space = ExtraAccountMetaList::size_of(
            InitializeExtraAccountMetaList::extra_account_metas()?.len()
        ).unwrap(),
        payer = payer
    )]
    pub extra_account_meta_list: AccountInfo<'info>,
    pub mint: InterfaceAccount<'info, Mint>,
    pub system_program: Program<'info, System>,
}
```

In this context, we are passing all the accounts needed to set up the transfer hook metadata:

- payer: The address paying for the initialization. He will be a signer of the transaction, and we mark his account as mutable as we will be deducting lamports from this account.

- extra_account_meta_list: The account that will store the extra metadata required for the transfer hook. This account is derived from the byte representation of "extra-account-metas" and the mint's public key, and its space is computed dynamically from the number of extra accounts we define.

- mint: The Token 2022 mint that will have the transfer hook enabled.

- system_program: Program responsible for the initialization of any new account.

### We then implement some functionality for our InitializeExtraAccountMetaList context:

```rust
impl<'info> InitializeExtraAccountMetaList<'info> {
    pub fn extra_account_metas() -> Result<Vec<ExtraAccountMeta>> {
        Ok(vec![ExtraAccountMeta::new_with_seeds(
            &[
                Seed::Literal { bytes: b"whitelist".to_vec() },
                Seed::AccountKey { index: 3 }, // index 3 is the owner of the source token account
            ],
            false,
            false,
        ).unwrap()])
    }
}
```

In here, we define the extra accounts that will be required during transfer hook execution. Rather than pre-computing a static PDA address, we register a dynamic seed resolution rule using `ExtraAccountMeta::new_with_seeds`. At transfer time, the SPL Token 2022 runtime will evaluate these seeds — the literal bytes `"whitelist"` followed by the public key at index 3 in the transfer accounts (the owner of the source token account) — to derive the correct per-user Whitelist PDA on the fly. This is the key design difference from a global whitelist: the PDA resolved at runtime is unique to whoever is sending the tokens.

---

### The transfer hook will validate every token transfer:

```rust
#[derive(Accounts)]
pub struct TransferHook<'info> {
    #[account(token::mint = mint, token::authority = owner)]
    pub source_token: InterfaceAccount<'info, TokenAccount>,
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(token::mint = mint)]
    pub destination_token: InterfaceAccount<'info, TokenAccount>,
    /// CHECK: source token account owner, can be SystemAccount or PDA owned by another program
    pub owner: UncheckedAccount<'info>,
    /// CHECK: ExtraAccountMetaList Account,
    #[account(seeds = [b"extra-account-metas", mint.key().as_ref()], bump)]
    pub extra_account_meta_list: UncheckedAccount<'info>,
    #[account(seeds = [b"whitelist", owner.key().as_ref()], bump = whitelist.bump)]
    pub whitelist: Account<'info, Whitelist>,
}
```

In this context, we are passing all the accounts needed for transfer validation:

- source_token: The token account from which tokens are being transferred. We validate that it belongs to the correct mint and is owned by the `owner` account.

- mint: The Token 2022 mint being transferred.

- destination_token: The token account to which tokens are being transferred. We validate that it belongs to the correct mint.

- owner: The owner of the source token account. This can be a system account or a PDA owned by another program.

- extra_account_meta_list: The metadata account that stores the dynamic seed rules for this transfer hook, derived from "extra-account-metas" and the mint.

- whitelist: The per-user Whitelist PDA derived from "whitelist" and the `owner`'s public key. The bump is verified against the stored bump in the account data, ensuring the correct PDA is resolved.

### We then implement some functionality for our TransferHook context:

```rust
impl<'info> TransferHook<'info> {
    pub fn transfer_hook(&mut self, _amount: u64) -> Result<()> {
        self.check_is_transferring()?;
        if self.whitelist.is_whitelisted {
            msg!("Transfer allowed: The address is whitelisted");
        } else {
            panic!("TransferHook: Address is not whitelisted");
        }
        Ok(())
    }
}
```

In this implementation, we first verify that the hook is being called during an actual transfer operation using `check_is_transferring`, which inspects the `TransferHookAccount` extension on the source token account. Then we read the `is_whitelisted` flag from the per-user Whitelist PDA. If the flag is `true`, the transfer is allowed to proceed; otherwise it panics, reverting the entire token transfer. Because the Whitelist PDA is derived from the sender's public key, each transfer is validated against the individual sender's whitelist entry rather than a shared list.

---

### The schedule instruction queues an automatic whitelist removal task on TukTuk. For that, we create the following context:

```rust
#[derive(Accounts)]
pub struct Schedule<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(
        mut @WhitelistError::NotInitialized,
        seeds = [b"whitelist", user_wallet.key().as_ref()],
        bump = whitelist.bump,
    )]
    pub whitelist: Account<'info, Whitelist>,
    #[account(
        mut,
        constraint = user_wallet.key() == whitelist.address.key() @WhitelistError::WrongAccount
    )]
    pub user_wallet: SystemAccount<'info>,
    #[account(mut)]
    pub task_queue: UncheckedAccount<'info>,
    pub task_queue_authority: UncheckedAccount<'info>,
    #[account(mut)]
    pub task: UncheckedAccount<'info>,
    #[account(mut, seeds = [b"queue_authority"], bump)]
    pub queue_authority: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
    pub tuktuk_program: Program<'info, Tuktuk>,
}
```

In this context, we are passing all the accounts needed to queue a whitelist removal task on TukTuk:

- user: The user who initiates the scheduling. He will be a signer of the transaction, and we mark his account as mutable as we will be deducting lamports to fund the task.

- whitelist: The per-user Whitelist PDA for `user_wallet`. The bump is verified against the stored bump in the account, and `@WhitelistError::NotInitialized` ensures the account exists before we attempt to schedule against it.

- user_wallet: The system account whose whitelist entry will be removed by the scheduled task. The constraint enforces that it matches the `address` stored inside the Whitelist PDA.

- task_queue: The TukTuk task queue where the task will be submitted.

- task_queue_authority: The authority PDA for the task queue, used to verify scheduling permissions.

- task: The account that will hold the queued task data, initialized during the CPI call.

- queue_authority: A PDA derived from "queue_authority" that acts as the program's signing authority when interacting with the TukTuk program via CPI.

- system_program: Program responsible for account creation.

- tuktuk_program: The TukTuk program that processes task queue operations.

### We then implement some functionality for our Schedule context:

```rust
impl<'info> Schedule<'info> {
    pub fn schedule(&mut self, task_id: u16, bumps: ScheduleBumps) -> Result<()> {
        let (compiled_tx, _) = compile_transaction(
            vec![Instruction {
                program_id: crate::ID,
                accounts: crate::__client_accounts_whitelist_operations::WhitelistOperations {
                    admin: self.user.key(),
                    system_program: self.system_program.key(),
                    user_wallet: self.user_wallet.key(),
                    whitelist: self.whitelist.key(),
                }
                .to_account_metas(None)
                .to_vec(),
                data: crate::instruction::RemoveFromWhitelist {
                    user: self.user_wallet.key(),
                }
                .data(),
            }],
            vec![],
        ).unwrap();

        queue_task_v0(
            CpiContext::new_with_signer(
                self.tuktuk_program.to_account_info(),
                QueueTaskV0 {
                    payer: self.user.to_account_info(),
                    queue_authority: self.queue_authority.to_account_info(),
                    task_queue: self.task_queue.to_account_info(),
                    task_queue_authority: self.task_queue_authority.to_account_info(),
                    task: self.task.to_account_info(),
                    system_program: self.system_program.to_account_info(),
                },
                &[&["queue_authority".as_bytes(), &[bumps.queue_authority]]],
            ),
            QueueTaskArgsV0 {
                trigger: TriggerV0::Now,
                transaction: TransactionSourceV0::CompiledV0(compiled_tx),
                crank_reward: Some(10000006),
                free_tasks: 1,
                id: task_id,
                description: "solver crank".to_string(),
            },
        )?;
        Ok(())
    }
}
```

In this implementation, we first compile a `remove_from_whitelist` instruction targeting the specific user's Whitelist PDA into TukTuk's compiled transaction format using `compile_transaction`. We then perform a CPI call to the TukTuk program's `queue_task_v0` instruction, signing with the program's `queue_authority` PDA. The task is configured with `TriggerV0::Now` so TukTuk crankers pick it up and execute it as soon as possible, a crank reward of 10,000,006 lamports to incentivize crankers, and a unique task ID provided by the caller. Once a cranker executes the task, the `remove_from_whitelist` instruction fires, setting `is_whitelisted` to `false` on the user's PDA without any further intervention from the admin.

---

## Flow of Actions

Here is the step-by-step flow to set up and run the Scheduled Whitelist Transfer Hook:

### 1. Build and deploy the program

```bash
anchor build
anchor deploy
```

### 2. Create a TukTuk Task Queue

Use the TukTuk CLI to create a task queue on devnet:

```bash
tuktuk -u https://api.devnet.solana.com task-queue create \
  --name inspiration_gx \
  --capacity 11 \
  --funding-amount 1000000000 \
  --queue-authority 6nqQQzui5aJGKCsAkbzx8RB87tcmLvHoz5ajpVpYFgDd \
  --min-crank-reward 10000005 \
  --stale-task-age 4320000
```

This creates a task queue named `scheduled-whitelist` with a capacity of 11 tasks, funded with 1 SOL, and a minimum crank reward of ~0.01 SOL.

### 3. Add a Queue Authority

Add the program's `queue_authority` PDA as a queue authority so it can submit tasks via CPI:

```bash
tuktuk --url https://api.devnet.solana.com task-queue add-queue-authority \
  --task-queue-name inspiration_gx \
  --queue-authority 6nqQQzui5aJGKCsAkbzx8RB87tcmLvHoz5ajpVpYFgDd
```

### 4. Initialize a per-user Whitelist PDA

Call the `initialize_whitelist` instruction to create the Whitelist PDA for a given wallet. This sets `is_whitelisted` to `false` and binds the PDA to the wallet's public key:

```bash
anchor test --skip-local-validator
```

The test file calls `program.methods.initializeWhitelist(userWallet)` which creates the PDA at seeds `["whitelist", userWallet]`.

### 5. Add the address to the whitelist

Call the `add_to_whitelist` instruction to flip `is_whitelisted` to `true` for the user's PDA:

```bash
anchor test --skip-local-validator
```

The test file calls `program.methods.addToWhitelist(userWallet)` which sets `is_whitelisted = true` on the user's Whitelist PDA.

### 6. Initialize the transfer hook extra account metas

Call the `initialize_transfer_hook` instruction to create the `extra_account_meta_list` PDA for the mint. This registers the dynamic seed resolution rule that maps each sender to their own Whitelist PDA at transfer time:

```bash
anchor test --skip-local-validator
```

The test file calls `program.methods.initializeTransferHook()` with the mint and the `extraAccountMetaList` PDA.

### 7. Transfer tokens

Send a Token 2022 transfer. The runtime will automatically invoke the transfer hook, resolve the sender's per-user Whitelist PDA using the registered seed rules, and validate `is_whitelisted`. If the flag is `true`, the transfer proceeds; if `false`, it reverts.

```bash
anchor test --skip-local-validator
```

### 8. Schedule whitelist removal

Call the `schedule_remove_from_whitelist` instruction to queue an automatic `remove_from_whitelist` task on TukTuk. TukTuk crankers will pick up the task and execute it, setting `is_whitelisted` back to `false` for the user's PDA:

```bash
anchor test --skip-local-validator
```

The test file calls `program.methods.scheduleRemoveFromWhitelist(taskId)` with the task queue, task queue authority, task account, and queue authority PDA. After crankers execute the task, the user's Whitelist PDA will have `is_whitelisted = false` and any subsequent transfer will be rejected by the hook.

---

This Scheduled Whitelist Transfer Hook demonstrates how to combine the SPL Token 2022 Transfer Hook interface with TukTuk's decentralized task queue to build per-user access control over token transfers, where whitelist entries can be granted and then automatically revoked on-chain — all without relying on centralized off-chain infrastructure.
