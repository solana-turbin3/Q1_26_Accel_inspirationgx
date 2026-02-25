use std::str::FromStr;

use anchor_lang::solana_program::instruction::Instruction;
use anchor_lang::{prelude::*, InstructionData};
use tuktuk_program::{
    compile_transaction,
    tuktuk::{
        cpi::{
            accounts::{InitializeTaskQueueV0, QueueTaskV0},
            initialize_task_queue_v0, queue_task_v0,
        },
        program::Tuktuk,
        types::TriggerV0,
    },
    types::QueueTaskArgsV0,
    TransactionSourceV0,
};

use crate::errors::WhitelistError;
use crate::state::Whitelist;

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
    /// CHECK: Don't need to parse this account, just using it in CPI
    pub task_queue: UncheckedAccount<'info>,
    /// CHECK: Don't need to parse this account, just using it in CPI
    pub task_queue_authority: UncheckedAccount<'info>,
    /// CHECK: Initialized in CPI
    #[account(mut)]
    pub task: UncheckedAccount<'info>,
    /// CHECK: Via seeds
    #[account(
        mut,
        seeds = [b"queue_authority"],
        bump
    )]
    pub queue_authority: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
    pub tuktuk_program: Program<'info, Tuktuk>,
}

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
        )
        .unwrap();

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
