use anchor_lang::prelude::*;
use anchor_spl::token::{self, Transfer};
use crate::{BuyTani, BuyTaniReferred};
use crate::errors::Z4Error;
use crate::events::{TokenSaleEvent, ReferralEvent};

pub fn handler(ctx: Context<BuyTani>, usdt_amount: u64) -> Result<()> {
    let config = &mut ctx.accounts.platform_config;

    require!(config.token_sale_active, Z4Error::TokenSaleInactive);
    require!(usdt_amount > 0, Z4Error::InvalidUsdtAmount);

    // tani_per_usdt disimpan dalam skala ×10 (contoh: 17 = 1.7 TANI/USDT)
    // Formula: usdt_raw × rate × 100 = tani_raw (6 desimal USDT → 9 desimal TANI, dibagi 10 dari skala)
    let tani_amount = usdt_amount
        .checked_mul(config.tani_per_usdt)
        .ok_or(Z4Error::Overflow)?
        .checked_mul(100)
        .ok_or(Z4Error::Overflow)?;

    require!(
        ctx.accounts.sale_inventory.amount >= tani_amount,
        Z4Error::InsufficientInventory
    );

    // Transfer USDT dari buyer ke treasury
    token::transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.buyer_usdt_account.to_account_info(),
                to: ctx.accounts.usdt_treasury.to_account_info(),
                authority: ctx.accounts.buyer.to_account_info(),
            },
        ),
        usdt_amount,
    )?;

    // Transfer $TANI dari sale inventory ke buyer (via sale_authority PDA)
    let sale_authority_bump = ctx.bumps.sale_authority;
    let seeds = &[b"sale_authority".as_ref(), &[sale_authority_bump]];
    let signer_seeds = &[&seeds[..]];

    token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.sale_inventory.to_account_info(),
                to: ctx.accounts.buyer_tani_account.to_account_info(),
                authority: ctx.accounts.sale_authority.to_account_info(),
            },
            signer_seeds,
        ),
        tani_amount,
    )?;

    config.total_tani_sold = config.total_tani_sold
        .checked_add(tani_amount)
        .ok_or(Z4Error::Overflow)?;

    emit!(TokenSaleEvent {
        wallet: ctx.accounts.buyer.key(),
        usdt_amount,
        tani_received: tani_amount,
        rate: config.tani_per_usdt,
        timestamp: Clock::get()?.unix_timestamp,
    });

    msg!("Token Sale: {} USDT → {} TANI wallet={}", usdt_amount, tani_amount, ctx.accounts.buyer.key());
    Ok(())
}

/// buy_tani dengan referral — 5% USDT ke referrer, 95% ke treasury.
/// Buyer tetap dapat TANI penuh dari 100% usdt_amount.
pub fn handler_referred(ctx: Context<BuyTaniReferred>, usdt_amount: u64) -> Result<()> {
    let config = &mut ctx.accounts.platform_config;

    require!(config.token_sale_active, Z4Error::TokenSaleInactive);
    require!(usdt_amount > 0, Z4Error::InvalidUsdtAmount);

    let tani_amount = usdt_amount
        .checked_mul(config.tani_per_usdt)
        .ok_or(Z4Error::Overflow)?
        .checked_mul(100)
        .ok_or(Z4Error::Overflow)?;

    require!(
        ctx.accounts.sale_inventory.amount >= tani_amount,
        Z4Error::InsufficientInventory
    );

    // Hitung 5% referral bonus
    let referral_bonus = usdt_amount
        .checked_mul(5)
        .ok_or(Z4Error::Overflow)?
        .checked_div(100)
        .ok_or(Z4Error::Overflow)?;

    let treasury_amount = usdt_amount
        .checked_sub(referral_bonus)
        .ok_or(Z4Error::Overflow)?;

    // Transfer 95% USDT → treasury
    token::transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.buyer_usdt_account.to_account_info(),
                to: ctx.accounts.usdt_treasury.to_account_info(),
                authority: ctx.accounts.buyer.to_account_info(),
            },
        ),
        treasury_amount,
    )?;

    // Transfer 5% USDT → referrer
    token::transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.buyer_usdt_account.to_account_info(),
                to: ctx.accounts.referrer_usdt_account.to_account_info(),
                authority: ctx.accounts.buyer.to_account_info(),
            },
        ),
        referral_bonus,
    )?;

    // Transfer TANI → buyer (penuh, berdasarkan 100% usdt_amount)
    let sale_authority_bump = ctx.bumps.sale_authority;
    let seeds = &[b"sale_authority".as_ref(), &[sale_authority_bump]];
    let signer_seeds = &[&seeds[..]];

    token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.sale_inventory.to_account_info(),
                to: ctx.accounts.buyer_tani_account.to_account_info(),
                authority: ctx.accounts.sale_authority.to_account_info(),
            },
            signer_seeds,
        ),
        tani_amount,
    )?;

    config.total_tani_sold = config.total_tani_sold
        .checked_add(tani_amount)
        .ok_or(Z4Error::Overflow)?;

    let referrer_key = ctx.accounts.referrer_usdt_account.owner;

    emit!(ReferralEvent {
        buyer: ctx.accounts.buyer.key(),
        referrer: referrer_key,
        usdt_amount,
        referral_bonus,
        tani_received: tani_amount,
        timestamp: Clock::get()?.unix_timestamp,
    });

    msg!(
        "Token Sale (Referral): {} USDT → {} TANI | referrer={} bonus={} USDT",
        usdt_amount, tani_amount, referrer_key, referral_bonus
    );
    Ok(())
}
