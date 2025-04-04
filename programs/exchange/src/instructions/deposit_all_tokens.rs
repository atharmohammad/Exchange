use crate::constants::{AUTHORITY, PREFIX};
use crate::errors::ExchangeError;
use crate::{curve::constant_product::*, Pool};
use anchor_lang::prelude::*;
use anchor_spl::token::{mint_to, transfer, Mint, MintTo, Token, TokenAccount, Transfer};

#[derive(Accounts)]
pub struct DepositAllTokens<'info> {
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
            creator.key().as_ref()
        ],
        bump
    )]
    pub pool: Box<Account<'info, Pool>>,

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
        token::mint=pool.token_a_mint,
        token::authority=user.key()
    )]
    pub user_token_a_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        token::mint=pool.token_b_mint,
        token::authority=user.key()
    )]
    pub user_token_b_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        token::mint=pool_mint,
        token::authority=user.key()
    )]
    pub user_pool_token_receipt: Account<'info, TokenAccount>,

    #[account(
        mut,
        address=pool.mint @ ExchangeError::InvalidMint,
        mint::authority = pool_authority,
        mint::freeze_authority = pool_authority,
        mint::decimals = 9
    )]
    pub pool_mint: Account<'info, Mint>,

    #[account(token::mint=pool_mint)]
    pub pool_token_fee_account: Box<Account<'info, TokenAccount>>,

    #[account(mut)]
    pub user: Signer<'info>,

    /// CHECK: Checked in pool seeds
    creator: AccountInfo<'info>,

    pub system_program: Program<'info, System>,

    pub token_program: Program<'info, Token>,
}

pub fn deposit_all_tokens_in(
    ctx: Context<DepositAllTokens>,
    min_pool_tokens: u64,
    max_token_a: u64,
    max_token_b: u64,
) -> Result<()> {
    let pool_mint_account = &ctx.accounts.pool_mint;

    let (token_a_amount, token_b_amount) = calculate_trade_tokens_propotional_to_pool_tokens(
        min_pool_tokens as u128,
        pool_mint_account.supply as u128,
        ctx.accounts.pool_token_a_account.amount as u128,
        ctx.accounts.pool_token_b_account.amount as u128,
    )
    .unwrap();

    if token_a_amount as u64 > max_token_a || token_b_amount as u64 > max_token_b {
        return Err(ExchangeError::SlippageExceeded.into());
    }

    let transfer_token_a_accounts = Transfer {
        from: ctx.accounts.user_token_a_account.to_account_info(),
        to: ctx.accounts.pool_token_a_account.to_account_info(),
        authority: ctx.accounts.user.to_account_info(),
    };

    let transfer_token_a_context = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        transfer_token_a_accounts,
    );

    transfer(transfer_token_a_context, token_a_amount as u64)?;

    let transfer_token_b_accounts = Transfer {
        from: ctx.accounts.user_token_b_account.to_account_info(),
        to: ctx.accounts.pool_token_b_account.to_account_info(),
        authority: ctx.accounts.user.to_account_info(),
    };
    let transfer_token_b_context = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        transfer_token_b_accounts,
    );
    transfer(transfer_token_b_context, token_b_amount as u64)?;

    let mint_to_accounts = MintTo {
        to: ctx.accounts.user_pool_token_receipt.to_account_info(),
        mint: ctx.accounts.pool_mint.to_account_info(),
        authority: ctx.accounts.pool_authority.to_account_info(),
    };

    let pool_key = ctx.accounts.pool.key();
    let signer_seeds = &[
        PREFIX,
        pool_key.as_ref(),
        AUTHORITY,
        &[ctx.bumps.pool_authority],
    ];

    let signer = &[&signer_seeds[..]];
    let mint_to_context = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        mint_to_accounts,
        signer,
    );
    mint_to(mint_to_context, min_pool_tokens)?;

    Ok(())
}
