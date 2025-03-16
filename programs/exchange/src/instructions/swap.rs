use super::TradeDirection;
use crate::constants::{AUTHORITY, PREFIX};
use crate::errors::ExchangeError;
use crate::{curve::constant_product::*, Pool};
use anchor_lang::prelude::*;
use anchor_spl::token::{mint_to, transfer, Mint, MintTo, Token, TokenAccount, Transfer};
use anchor_spl::token_interface::spl_token_2022::cmp_pubkeys;

#[derive(Accounts)]
pub struct Swap<'info> {
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
            pool.token_a_mint.as_ref(),
            pool.token_b_mint.as_ref(),
            pool.creator.as_ref()
        ],
        bump
    )]
    pub pool: Account<'info, Pool>,

    /// Non-zero token A accoun
    #[account(
        address=pool.token_a @ ExchangeError::InvalidPoolTokenAccount,
        owner=pool_authority.key()
    )]
    pub pool_token_a_account: Account<'info, TokenAccount>,

    /// Non-zero token B accoun
    #[account(
        address=pool.token_b @ ExchangeError::InvalidPoolTokenAccount,
        owner=pool_authority.key()
    )]
    pub pool_token_b_account: Account<'info, TokenAccount>,

    #[account(owner=user.key())]
    pub user_source_token_account: Account<'info, TokenAccount>,

    #[account(owner=user.key())]
    pub user_destination_token_account: Account<'info, TokenAccount>,

    #[account(
        address=pool.mint @ ExchangeError::InvalidMint,
        owner=pool.key()
    )]
    pub pool_mint: Account<'info, Mint>,

    #[account(token::mint=pool_mint)]
    pub pool_token_fee_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub user: Signer<'info>,

    pub system_program: Program<'info, System>,

    pub token_program: Program<'info, Token>,
}

pub fn swap(ctx: Context<Swap>, source_amount: u64) -> Result<()> {
    let pool = &ctx.accounts.pool;
    let pool_mint_account = &ctx.accounts.pool_mint;
    let source_mint = &ctx.accounts.user_source_token_account.mint;
    let destination_mint = &ctx.accounts.user_destination_token_account.mint;

    if ctx.accounts.user_source_token_account.amount < source_amount {
        return Err(ExchangeError::NotEnoughFunds.into());
    }

    let trade_direction = if cmp_pubkeys(&source_mint.key(), &pool.token_a_mint) {
        TradeDirection::TokenAtoB
    } else {
        TradeDirection::TokenBtoA
    };

    let (pool_source_token_account, pool_destination_token_account) = match trade_direction {
        TradeDirection::TokenAtoB => (
            &ctx.accounts.pool_token_a_account,
            &ctx.accounts.pool_token_b_account,
        ),
        TradeDirection::TokenBtoA => (
            &ctx.accounts.pool_token_b_account,
            &ctx.accounts.pool_token_a_account,
        ),
    };

    if !cmp_pubkeys(&pool_source_token_account.mint.key(), &source_mint)
        || !cmp_pubkeys(
            &pool_destination_token_account.mint.key(),
            &destination_mint,
        )
    {
        return Err(ExchangeError::InvalidMint.into());
    }

    let (
        new_pool_source_amount,
        _new_pool_destination_amount,
        swapped_source_amount,
        swapped_destination_amount,
        owner_fee,
        _trading_fee,
    ) = calculate_swap_amounts(
        source_amount as u128,
        pool_source_token_account.amount as u128,
        pool_destination_token_account.amount as u128,
        &pool.fees,
    )?;

    // transfer the swapped amounts
    let source_transfer_accounts = Transfer {
        authority: ctx.accounts.user.to_account_info(),
        to: pool_source_token_account.to_account_info(),
        from: ctx.accounts.user_source_token_account.to_account_info(),
    };

    let source_transfer_context = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        source_transfer_accounts,
    );

    transfer(source_transfer_context, swapped_source_amount as u64)?;

    let destination_transfer_accounts = Transfer {
        authority: ctx.accounts.pool_authority.to_account_info(),
        to: ctx
            .accounts
            .user_destination_token_account
            .to_account_info(),
        from: pool_destination_token_account.to_account_info(),
    };

    let pool_key = ctx.accounts.pool.key();
    let signer_seeds = &[
        PREFIX,
        pool_key.as_ref(),
        AUTHORITY,
        &[ctx.bumps.pool_authority],
    ];

    let signer = &[&signer_seeds[..]];

    let destination_transfer_context = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        destination_transfer_accounts,
        signer,
    );
    transfer(
        destination_transfer_context,
        swapped_destination_amount as u64,
    )?;

    // mint the pool_tokens propotional to owner_fee to pool_fee_accoun
    let pool_tokens = calculate_pool_tokens_propotional_to_single_token_redeemed(
        owner_fee,
        new_pool_source_amount,
        pool_mint_account.supply as u128,
    )?;

    let pool_mint_to_fee_account = MintTo {
        authority: ctx.accounts.pool_authority.to_account_info(),
        mint: pool_mint_account.to_account_info(),
        to: ctx.accounts.pool_token_fee_account.to_account_info(),
    };

    let pool_mint_to_context = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        pool_mint_to_fee_account,
        signer,
    );
    mint_to(pool_mint_to_context, pool_tokens as u64)?;

    Ok(())
}
