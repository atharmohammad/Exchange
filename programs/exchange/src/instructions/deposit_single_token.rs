use crate::constants::{AUTHORITY, PREFIX};
use crate::errors::ExchangeError;
use crate::{curve::constant_product::*, Pool};
use anchor_lang::prelude::*;
use anchor_spl::token::{mint_to, transfer, Mint, MintTo, Token, TokenAccount, Transfer};
use anchor_spl::token_interface::spl_token_2022::cmp_pubkeys;

use super::TradeDirection;

#[derive(Accounts)]
pub struct DepositSingleToken<'info> {
    /// CHECK: Account seeds checked in constraints
    #[account(
        seeds=[
            PREFIX,
            pool.key().as_ref(),
            AUTHORITY
        ],
        bump
    )]
    pub pool_authority: AccountInfo<'info>,

    #[account(
        seeds=[
            PREFIX,
            pool_token_a_account.mint.as_ref(),
            pool_token_b_account.mint.as_ref(),
            pool.creator.as_ref()
        ],
        bump
    )]
    pub pool: Account<'info, Pool>,

    /// Non-zero token A account
    #[account(
        mut,
        address=pool.token_a @ ExchangeError::InvalidPoolTokenAccount,
        token::authority=pool_authority.key()
    )]
    pub pool_token_a_account: Account<'info, TokenAccount>,

    /// Non-zero token B account
    #[account(
        mut,
        address=pool.token_b @ ExchangeError::InvalidPoolTokenAccount,
        token::authority=pool_authority.key()
    )]
    pub pool_token_b_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        token::mint=source_mint,
        token::authority=user.key()
    )]
    pub user_source_token_account: Account<'info, TokenAccount>,

    pub source_mint: Account<'info, Mint>,

    #[account(
        mut,
        token::mint=pool.mint
    )]
    pub user_pool_token_receipt: Account<'info, TokenAccount>,

    #[account(
        mut,
        address=pool.mint @ ExchangeError::InvalidMint
    )]
    pub pool_mint: Account<'info, Mint>,

    #[account(mut)]
    pub user: Signer<'info>,

    pub system_program: Program<'info, System>,

    pub token_program: Program<'info, Token>,
}

pub fn deposit_single_token_in(ctx: Context<DepositSingleToken>, source_amount: u64) -> Result<()> {
    let source_mint = &ctx.accounts.source_mint;
    let pool = &ctx.accounts.pool;
    let user_source_token_account = &ctx.accounts.user_source_token_account;

    if !cmp_pubkeys(&source_mint.key(), &pool.token_a_mint)
        && !cmp_pubkeys(&source_mint.key(), &pool.token_b_mint)
    {
        return Err(ExchangeError::InvalidMint.into());
    }

    if user_source_token_account.amount < source_amount {
        return Err(ExchangeError::NotEnoughFunds.into());
    }

    let (_, pool_source_token_account) = if cmp_pubkeys(&source_mint.key(), &pool.token_a_mint) {
        (
            TradeDirection::TokenAtoB,
            &ctx.accounts.pool_token_a_account,
        )
    } else {
        (
            TradeDirection::TokenBtoA,
            &ctx.accounts.pool_token_b_account,
        )
    };

    let user_source_pool_tokens = calculate_pool_tokens_propotional_to_single_token_deposit(
        source_amount as u128,
        pool_source_token_account.amount as u128,
        ctx.accounts.pool_mint.supply as u128,
    )?;

    // transfer the source amount
    let source_amount_transfer_accounts = Transfer {
        to: pool_source_token_account.to_account_info(),
        from: user_source_token_account.to_account_info(),
        authority: ctx.accounts.user.to_account_info(),
    };

    let source_amount_transfer_context = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        source_amount_transfer_accounts,
    );
    transfer(source_amount_transfer_context, source_amount as u64)?;

    let pool_key = ctx.accounts.pool.key();
    let signer_seeds = &[
        PREFIX,
        pool_key.as_ref(),
        AUTHORITY,
        &[ctx.bumps.pool_authority],
    ];

    let signer = &[&signer_seeds[..]];
    // mint pool token propotional to deposited source amount
    let mint_pool_tokens_account = MintTo {
        to: ctx.accounts.user_pool_token_receipt.to_account_info(),
        mint: ctx.accounts.pool_mint.to_account_info(),
        authority: ctx.accounts.pool_authority.to_account_info(),
    };

    let mint_pool_tokens_context = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        mint_pool_tokens_account,
        signer,
    );
    mint_to(mint_pool_tokens_context, user_source_pool_tokens as u64)?;

    Ok(())
}
