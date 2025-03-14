use anchor_lang::prelude::*;

use crate::Fee;
#[account]
pub struct Pool {
    pub bump: u8,
    pub token_a: Pubkey,
    pub token_b: Pubkey,
    pub token_a_mint: Pubkey,
    pub token_b_mint: Pubkey,
    pub mint: Pubkey,
    pub creator: Pubkey,
    pub pool_fee_account: Pubkey,
    pub fees: Fee,
}

impl Pool {
    pub const MAX_SIZE: usize = 8 + 1 + 32 * 8 + Fee::MAX_SIZE;
    pub const INITIAL_POOL_TOKEN_SUPPLY: u64 = 1000_000_000;
}
