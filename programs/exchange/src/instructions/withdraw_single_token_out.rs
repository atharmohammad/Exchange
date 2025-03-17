use crate::constants::{AUTHORITY, PREFIX};
use crate::errors::ExchangeError;
use crate::{curve::constant_product::*, Pool};
use anchor_lang::prelude::*;
use anchor_spl::token::{burn, transfer, Burn, Mint, Token, TokenAccount, Transfer};
use anchor_spl::token_interface::spl_token_2022::cmp_pubkeys;

#[derive(Accounts)]
pub struct WithdrawSingleToken<'info> {
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

    // mint of token to be withdrawn
    pub source_mint: Account<'info, Mint>,

    #[account(
        mut,
        token::mint=pool_mint,
        token::authority=user.key()
    )]
    pub user_pool_token_receipt: Account<'info, TokenAccount>,

    #[account(
        mut,
        address=pool.mint @ ExchangeError::InvalidMint
    )]
    pub pool_mint: Account<'info, Mint>,

    #[account(mut, token::mint=pool_mint)]
    pub pool_token_fee_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub user: Signer<'info>,

    pub system_program: Program<'info, System>,

    pub token_program: Program<'info, Token>,
}

pub fn withdraw_single_token_out(
    ctx: Context<WithdrawSingleToken>,
    source_amount: u64,
) -> Result<()> {
    let pool = &ctx.accounts.pool;
    let user_pool_token_account = &ctx.accounts.user_pool_token_receipt;
    let user_source_token_account = &ctx.accounts.user_source_token_account;
    let source_mint_account = &ctx.accounts.source_mint;
    let pool_mint = &ctx.accounts.pool_mint;

    if user_source_token_account.amount < source_amount {
        return Err(ExchangeError::NotEnoughFunds.into());
    }

    let (pool_source_token_account, _) =
        if cmp_pubkeys(&source_mint_account.key(), &pool.token_a_mint) {
            (
                &ctx.accounts.pool_token_a_account,
                &ctx.accounts.pool_token_b_account,
            )
        } else {
            (
                &ctx.accounts.pool_token_b_account,
                &ctx.accounts.pool_token_a_account,
            )
        };

    let burn_pool_token_amount = calculate_pool_tokens_propotional_to_single_token_redeemed(
        source_amount as u128,
        pool_source_token_account.amount as u128,
        pool_mint.supply as u128,
    )? as u64;

    if user_pool_token_account.amount < burn_pool_token_amount {
        return Err(ExchangeError::NotEnoughFunds.into());
    }

    // Todo: transfer withdraw fee

    let pool_key = ctx.accounts.pool.key();
    let signer_seeds = &[
        PREFIX,
        pool_key.as_ref(),
        AUTHORITY,
        &[ctx.bumps.pool_authority],
    ];

    let signer = &[&signer_seeds[..]];
    // burn the pool tokens
    let burn_user_pool_tokens_accounts = Burn {
        mint: pool_mint.to_account_info(),
        from: user_pool_token_account.to_account_info(),
        authority: ctx.accounts.user.to_account_info(),
    };

    let burn_pool_tokens_context = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        burn_user_pool_tokens_accounts,
    );

    burn(burn_pool_tokens_context, burn_pool_token_amount)?;

    // transfer the withdrawal source amount
    let source_amount_transfer_accounts = Transfer {
        to: ctx.accounts.user_source_token_account.to_account_info(),
        from: pool_source_token_account.to_account_info(),
        authority: ctx.accounts.pool_authority.to_account_info(),
    };

    let source_amount_transfer_context = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        source_amount_transfer_accounts,
        signer,
    );
    transfer(source_amount_transfer_context, source_amount as u64)?;

    Ok(())
}
