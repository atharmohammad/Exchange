use crate::{errors::ExchangeError, fee::*};
use anchor_lang::Result;
use spl_math::{checked_ceil_div::CheckedCeilDiv, precise_number::PreciseNumber};

// Constant product swap : (A+A') * (B-B') = invariant
pub fn calculate_swap_amounts(
    source_amount: u128,
    pool_source_amount: u128,
    pool_destination_amount: u128,
    fee: &Fee,
) -> Result<(u128, u128, u128, u128, u128, u128)> {
    // Calculate the fee
    let trading_fee = calculate_fee(
        source_amount,
        fee.trade_fee_numerator,
        fee.trade_fee_denominator,
    )
    .ok_or(ExchangeError::NumeralOverflow)?;

    let owner_fee = calculate_fee(
        source_amount,
        fee.owner_trade_fee_numerator,
        fee.owner_trade_fee_denominator,
    )
    .ok_or(ExchangeError::NumeralOverflow)?;

    let total_fee = trading_fee
        .checked_add(owner_fee)
        .ok_or(ExchangeError::NumeralOverflow)?;
    let source_amount_after_fee = source_amount
        .checked_sub(total_fee)
        .ok_or(ExchangeError::NumeralOverflow)?;

    // invariant = (A*B)
    let invariant = pool_source_amount
        .checked_mul(pool_destination_amount)
        .ok_or(ExchangeError::NumeralOverflow)?;
    // A + A'
    let total_source_amount = pool_source_amount
        .checked_add(source_amount_after_fee)
        .ok_or(ExchangeError::NumeralOverflow)?;
    // B - B' = invariant/(A+A');
    let (total_destination_amount, total_source_amount) = invariant
        .checked_ceil_div(total_source_amount)
        .ok_or(ExchangeError::NumeralOverflow)?;

    // B' = B - invariant/(A+A')
    let swapped_destination_amount = pool_destination_amount
        .checked_sub(total_destination_amount)
        .ok_or(ExchangeError::NumeralOverflow)?;

    // A' = total_source - A
    let swapped_source_amount_with_fee = total_source_amount
        .checked_sub(pool_source_amount)
        .unwrap()
        .checked_add(total_fee)
        .ok_or(ExchangeError::NumeralOverflow)?;

    let new_pool_source_amount = source_amount
        .checked_add(swapped_source_amount_with_fee)
        .ok_or(ExchangeError::NumeralOverflow)?;
    let new_pool_destination_amount = pool_destination_amount
        .checked_sub(swapped_destination_amount)
        .ok_or(ExchangeError::NumeralOverflow)?;

    Ok((
        new_pool_source_amount,
        new_pool_destination_amount,
        swapped_source_amount_with_fee,
        swapped_destination_amount,
        owner_fee,
        trading_fee,
    ))
}

/*
    P ~ sqrt(A * B)
    P_new = [ P * sqrt((A' +  A) * (B' + B)) / sqrt(A * B) ]
    P' = P_new - P
    P' = [ P * sqrt((A' + A) * (B' + B)) / sqrt(A * B) ] - P
    P' = P * [ sqrt([(A'+ A) * (B' + B)] / A * B ) - 1 ]

    When deposit single token, B' = 0

    P' = P * [sqrt((A'+ A) / A) - 1]
*/

pub fn calculate_pool_tokens_propotional_to_single_token_deposit(
    source_amount: u128,
    pool_source_amount: u128,
    pool_supply: u128,
) -> Result<u128> {
    let source_amount =
        PreciseNumber::new(source_amount).ok_or(ExchangeError::FailedToCreatePreciseNumber)?;
    let pool_source_amount =
        PreciseNumber::new(pool_source_amount).ok_or(ExchangeError::FailedToCreatePreciseNumber)?;

    let pool_supply =
        PreciseNumber::new(pool_supply).ok_or(ExchangeError::FailedToCreatePreciseNumber)?;
    let one = PreciseNumber::new(1).ok_or(ExchangeError::FailedToCreatePreciseNumber)?;

    let new_pool_source_amount = source_amount
        .checked_add(&pool_source_amount)
        .ok_or(ExchangeError::NumeralOverflow)?;
    let ratio_deposited = new_pool_source_amount
        .checked_div(&pool_source_amount)
        .ok_or(ExchangeError::NumeralOverflow)?;

    let ratio = ratio_deposited
        .sqrt()
        .ok_or(ExchangeError::NumeralOverflow)?
        .checked_sub(&one)
        .ok_or(ExchangeError::NumeralOverflow)?;
    let result_amount = pool_supply
        .checked_mul(&ratio)
        .ok_or(ExchangeError::NumeralOverflow)?;

    let propotional_pool_tokens = result_amount
        .to_imprecise()
        .ok_or(ExchangeError::NumeralOverflow)?;
    Ok(propotional_pool_tokens)
}

/*
    P ~ sqrt(A * B)
    P_new = [ P * sqrt((A -  A) * (B - B')) / sqrt(A * B) ]
    P' = P_new - P
    P' = [ P * sqrt((A - A') * (B - B')) / sqrt(A * B) ] - P
    P' = P * [ sqrt([(A - A') * (B - B')] / A * B ) - 1 ]

    When withdraw single token, B' = 0

    P' = P * [sqrt((A - A') / A) - 1]
*/

pub fn calculate_pool_tokens_propotional_to_single_token_redeemed(
    source_amount: u128,
    pool_source_amount: u128,
    pool_supply: u128,
) -> Result<u128> {
    let source_amount =
        PreciseNumber::new(source_amount).ok_or(ExchangeError::FailedToCreatePreciseNumber)?;
    let pool_source_amount =
        PreciseNumber::new(pool_source_amount).ok_or(ExchangeError::FailedToCreatePreciseNumber)?;
    let pool_supply =
        PreciseNumber::new(pool_supply).ok_or(ExchangeError::FailedToCreatePreciseNumber)?;
    let one = PreciseNumber::new(1).ok_or(ExchangeError::FailedToCreatePreciseNumber)?;

    let new_pool_source_amount = pool_source_amount
        .checked_sub(&source_amount)
        .ok_or(ExchangeError::NumeralOverflow)?;
    let ratio_redeemed = new_pool_source_amount
        .checked_div(&pool_source_amount)
        .ok_or(ExchangeError::NumeralOverflow)?;

    let ratio = ratio_redeemed
        .sqrt()
        .ok_or(ExchangeError::NumeralOverflow)?
        .checked_sub(&one)
        .ok_or(ExchangeError::NumeralOverflow)?;
    let result_amount = pool_supply
        .checked_mul(&ratio)
        .ok_or(ExchangeError::NumeralOverflow)?;

    let propotional_pool_tokens = result_amount
        .to_imprecise()
        .ok_or(ExchangeError::NumeralOverflow)?;
    Ok(propotional_pool_tokens)
}

pub fn convert_pool_tokens_to_trade_tokens(
    pool_token_amount: u128,
    pool_token_supply: u128,
    pool_token_a: u128,
    pool_token_b: u128,
) -> Result<(u128, u128)> {
    let token_a = pool_token_amount
        .checked_mul(pool_token_a)
        .unwrap()
        .checked_div(pool_token_supply)
        .unwrap();

    let token_b = pool_token_amount
        .checked_mul(pool_token_b)
        .unwrap()
        .checked_div(pool_token_supply)
        .unwrap();

    Ok((token_a, token_b))
}

pub fn calculate_fee(
    source_amount: u128,
    fee_numerator: u64,
    fee_denominator: u64,
) -> Option<u128> {
    let fee: u128 = source_amount
        .checked_mul(fee_numerator as u128)?
        .checked_div(fee_denominator as u128)?;

    if fee == 0 {
        Some(1)
    } else {
        Some(fee)
    }
}
