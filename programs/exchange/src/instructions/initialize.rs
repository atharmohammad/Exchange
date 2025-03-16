use crate::constants::{AUTHORITY, PREFIX};
use crate::errors::*;
use crate::pool::Pool;
use crate::Fee;

use anchor_lang::prelude::*;
use anchor_lang::Accounts;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::mint_to;
use anchor_spl::token::MintTo;
use anchor_spl::token::Token;
use anchor_spl::token::{Mint, TokenAccount};
use anchor_spl::token_2022::spl_token_2022::cmp_pubkeys;

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
    pub pool: Box<Account<'info, Pool>>,

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

    /// Non-zero token A account
    #[account(token::authority=pool_authority.key())]
    pub token_a: Box<Account<'info, TokenAccount>>,

    /// Non-zero token B account
    #[account(token::authority=pool_authority.key())]
    pub token_b: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        mint::authority = pool_authority,
        mint::freeze_authority = pool_authority,
        mint::decimals = 9
    )]
    pub pool_mint: Box<Account<'info, Mint>>,

    /// pool token reciept as per the token A|B input
    #[account(
        mut,
        token::authority = creator,
        token::mint = pool_mint
    )]
    pub user_pool_token_receipt: Box<Account<'info, TokenAccount>>,

    #[account(
        init_if_needed,
        payer=creator,
        associated_token::authority = pool_authority,
        associated_token::mint = pool_mint
    )]
    pub pool_fee_account: Box<Account<'info, TokenAccount>>,

    #[account(mut)]
    pub creator: Signer<'info>,

    pub system_program: Program<'info, System>,

    pub token_program: Program<'info, Token>,

    pub associated_token_program: Program<'info, AssociatedToken>,
}

pub fn initialize(ctx: Context<InitializePool>, fees: Fee) -> Result<()> {
    let pool_mint = &ctx.accounts.pool_mint;
    let pool_authority = &ctx.accounts.pool_authority;

    let token_a = &ctx.accounts.token_a;
    let token_b = &ctx.accounts.token_b;

    if cmp_pubkeys(&token_a.mint, &token_b.mint) {
        return Err(ExchangeError::SameTokenMints.into());
    }

    let pool = &mut ctx.accounts.pool;
    pool.fees = fees;
    pool.token_a = token_a.key();
    pool.token_b = token_b.key();
    pool.token_a_mint = ctx.accounts.token_a.mint;
    pool.token_b_mint = ctx.accounts.token_b.mint;
    pool.mint = pool_mint.key();
    pool.creator = ctx.accounts.creator.key();
    pool.fee_account = ctx.accounts.pool_fee_account.key();

    // todo: validate fees

    let initial_supply: u64 = Pool::INITIAL_POOL_TOKEN_SUPPLY;
    let bump = ctx.bumps.pool;
    pool.bump = bump;

    let pool_key = pool.key();
    let signer_seeds = &[
        PREFIX,
        pool_key.as_ref(),
        AUTHORITY,
        &[ctx.bumps.pool_authority],
    ];
    let signer = &[&signer_seeds[..]];

    let cpi_accounts = MintTo {
        mint: pool_mint.to_account_info(),
        to: ctx.accounts.user_pool_token_receipt.to_account_info(),
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
