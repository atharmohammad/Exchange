use crate::constants::{AUTHORITY, PREFIX};
use crate::errors::ExchangeError;
use crate::{curve::constant_product::*, Pool};
use anchor_lang::prelude::*;
use anchor_spl::token::{mint_to, transfer, Mint, MintTo, Token, TokenAccount, Transfer};
use anchor_spl::token_interface::spl_token_2022::cmp_pubkeys;

use super::TradeDirection;

#[derive(Accounts)]
pub struct DepositSingleToken<'info> {
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

    #[account(
        token::mint=source_mint,
        owner=user.key()
    )]
    pub user_source_token_account: Account<'info, TokenAccount>,

    pub source_mint: Account<'info, Mint>,

    #[account(token::mint=pool.mint)]
    pub pool_token_recepient_account: Account<'info, TokenAccount>,

    #[account(
        address=pool.mint @ ExchangeError::InvalidMint,
        owner=pool.key()
    )]
    pub pool_mint: Account<'info, Mint>,

    #[account(token::mint=pool.mint)]
    pub pool_token_fee_account: Account<'info, TokenAccount>,

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

    let user_source_pool_tokens = calculate_deposit_single_token_in(
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
        to: user_source_token_account.to_account_info(),
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
