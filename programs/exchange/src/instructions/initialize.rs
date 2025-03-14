use crate::constants::{AUTHORITY, PREFIX};
use crate::errors::*;
use crate::pool::Pool;
use crate::Fee;

use anchor_lang::prelude::*;
use anchor_lang::Accounts;
use anchor_spl::token::mint_to;
use anchor_spl::token::MintTo;
use anchor_spl::token::Token;
use anchor_spl::token::{Mint, TokenAccount};

#[derive(Accounts)]
pub struct InitializePool<'info> {
    #[account(
        init,
        seeds=[
            PREFIX,
            token_a.mint.key().as_ref(),
            token_b.mint.key().as_ref(),
            creator.key().as_ref()
        ],
        bump,
        payer=creator,
        space=Pool::MAX_SIZE
    )]
    pub pool: Account<'info, Pool>,

    #[account(
        seeds=[
            PREFIX,
            pool.key().as_ref(),
            AUTHORITY
        ],
        bump
    )]
    pub pool_authority: AccountInfo<'info>,

    /// Non-zero token A accoun
    #[account(owner=pool_authority.key())]
    pub token_a: Account<'info, TokenAccount>,

    /// Non-zero token B accoun
    #[account(owner=pool_authority.key())]
    pub token_b: Account<'info, TokenAccount>,

    // todo: check for mint address
    #[account(
        owner=pool.key(),
        constraint=pool_mint.supply != 0 @ ExchangeError::PoolMintSupplyNotZero,
        constraint=pool_mint.freeze_authority.is_some() @ ExchangeError::InvalidAuthority
    )]
    pub pool_mint: Account<'info, Mint>,

    /// pool token reciept as per the token A|B inpu
    #[account(token::mint=pool_mint)]
    pub pool_token_reciept_account: Account<'info, TokenAccount>,

    #[account(token::mint=pool_mint)]
    pub pool_token_fee_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub creator: Signer<'info>,

    pub system_program: Program<'info, System>,

    pub token_program: Program<'info, Token>,
}

pub fn initialize(ctx: Context<InitializePool>, fees: Fee) -> Result<()> {
    let pool_mint = &ctx.accounts.pool_mint;
    let pool_authority = &ctx.accounts.pool_authority;

    let pool = &mut ctx.accounts.pool;
    pool.fees = fees;
    pool.token_a = ctx.accounts.token_a.key();
    pool.token_b = ctx.accounts.token_b.key();
    pool.token_a_mint = ctx.accounts.token_a.mint;
    pool.token_b_mint = ctx.accounts.token_b.mint;
    pool.mint = pool_mint.key();
    pool.creator = ctx.accounts.creator.key();

    if pool_mint.mint_authority.is_none()
        || pool_mint.mint_authority.unwrap() != pool_authority.key()
    {
        return Err(ExchangeError::InvalidAuthority.into());
    }

    // todo: validate fees

    let initial_supply: u64 = Pool::INITIAL_POOL_TOKEN_SUPPLY;
    let bump = ctx.bumps.pool;
    let token_a_mint_key = ctx.accounts.token_a.mint.key();
    let token_b_mint_key = ctx.accounts.token_b.mint.key();
    let creator_key = ctx.accounts.creator.key();
    pool.bump = bump;

    let signer_seeds = &[
        PREFIX,
        token_a_mint_key.as_ref(),
        token_b_mint_key.as_ref(),
        creator_key.as_ref(),
        &[bump],
    ];
    let signer = &[&signer_seeds[..]];

    let cpi_accounts = MintTo {
        mint: pool_mint.to_account_info(),
        to: ctx.accounts.pool_token_reciept_account.to_account_info(),
        authority: pool_authority.to_account_info(),
    };

    let cpi_context = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        cpi_accounts,
        signer,
    );
    mint_to(cpi_context, initial_supply)?;

    Ok(())
}
