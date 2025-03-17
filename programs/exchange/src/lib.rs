#![allow(unexpected_cfgs)]

use anchor_lang::prelude::*;
mod constants;
mod curve;
mod errors;
mod instructions;
mod state;

use instructions::*;
use state::*;

declare_id!("HndsTUfB2AZbQifHN9WdKMMQqXghGVwmak2gy3oyzwqV");

#[program]
pub mod exchange {
    use super::*;

    pub fn initialize(ctx: Context<InitializePool>, fees: Fee) -> Result<()> {
        instructions::initialize(ctx, fees)
    }

    pub fn swap(ctx: Context<Swap>, source_amount: u64) -> Result<()> {
        instructions::swap(ctx, source_amount)
    }

    pub fn deposit_all_tokens_in(
        ctx: Context<DepositAllTokens>,
        pool_tokens: u64,
        max_token_a: u64,
        max_token_b: u64,
    ) -> Result<()> {
        instructions::deposit_all_tokens_in(ctx, pool_tokens, max_token_a, max_token_b)
    }

    pub fn deposit_single_token(
        ctx: Context<DepositSingleToken>,
        source_amount: u64,
    ) -> Result<()> {
        instructions::deposit_single_token_in(ctx, source_amount)
    }

    pub fn withdraw_single_token_out(
        ctx: Context<WithdrawSingleToken>,
        source_amount: u64
    ) -> Result<()> {
        instructions::withdraw_single_token_out(ctx, source_amount)
    }
}
