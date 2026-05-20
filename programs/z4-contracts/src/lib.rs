mod events;
mod state;
mod errors;
pub mod instructions;

use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};
use state::PlatformConfig;
use errors::Z4Error;

declare_id!("9AShqzX8Y1uHDHxuhUvd6ojTwFa2BiBKPKHMSBSmV8MJ");

const PLATFORM_CONFIG_SEED: &[u8] = b"platform_config";

#[program]
pub mod z4_contracts {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, tani_per_usdt: u64) -> Result<()> {
        let config = &mut ctx.accounts.platform_config;
        config.authority = ctx.accounts.authority.key();
        config.tani_mint = ctx.accounts.tani_mint.key();
        config.usdt_mint = ctx.accounts.usdt_mint.key();
        config.sale_inventory = ctx.accounts.sale_inventory.key();
        config.usdt_treasury = ctx.accounts.usdt_treasury.key();
        config.tani_per_usdt = tani_per_usdt;
        config.token_sale_active = true;
        config.total_tani_sold = 0;
        config.bump = ctx.bumps.platform_config;
        msg!("Z4 Platform initialized");
        Ok(())
    }

    pub fn update_rate(ctx: Context<AdminAction>, new_rate: u64) -> Result<()> {
        require!(new_rate > 0, Z4Error::InvalidRate);
        ctx.accounts.platform_config.tani_per_usdt = new_rate;
        msg!("Rate updated: {} TANI per USDT", new_rate);
        Ok(())
    }

    pub fn toggle_token_sale(ctx: Context<AdminAction>, active: bool) -> Result<()> {
        ctx.accounts.platform_config.token_sale_active = active;
        msg!("Token Sale: {}", if active { "AKTIF" } else { "NONAKTIF" });
        Ok(())
    }

    pub fn set_sale_inventory(ctx: Context<SetSaleInventory>) -> Result<()> {
        ctx.accounts.platform_config.sale_inventory = ctx.accounts.new_sale_inventory.key();
        msg!("Sale inventory updated: {}", ctx.accounts.new_sale_inventory.key());
        Ok(())
    }

    pub fn buy_tani(ctx: Context<BuyTani>, usdt_amount: u64) -> Result<()> {
        instructions::token_sale::handler(ctx, usdt_amount)
    }

    /// Beli TANI menggunakan referral link.
    /// 5% dari usdt_amount → referrer, 95% → treasury.
    /// Buyer tetap menerima TANI penuh berdasarkan 100% usdt_amount.
    pub fn buy_tani_referred(ctx: Context<BuyTaniReferred>, usdt_amount: u64) -> Result<()> {
        instructions::token_sale::handler_referred(ctx, usdt_amount)
    }
}

// ─── Contexts ──────────────────────────────────────────────────────────────

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        payer = authority,
        space = PlatformConfig::LEN,
        seeds = [PLATFORM_CONFIG_SEED],
        bump,
    )]
    pub platform_config: Account<'info, PlatformConfig>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub tani_mint: Account<'info, Mint>,
    pub usdt_mint: Account<'info, Mint>,
    #[account(mut)]
    pub sale_inventory: Account<'info, TokenAccount>,
    #[account(mut)]
    pub usdt_treasury: Account<'info, TokenAccount>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct AdminAction<'info> {
    #[account(
        mut,
        seeds = [PLATFORM_CONFIG_SEED],
        bump = platform_config.bump,
        has_one = authority,
    )]
    pub platform_config: Account<'info, PlatformConfig>,
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct SetSaleInventory<'info> {
    #[account(
        mut,
        seeds = [PLATFORM_CONFIG_SEED],
        bump = platform_config.bump,
        has_one = authority,
    )]
    pub platform_config: Account<'info, PlatformConfig>,
    pub authority: Signer<'info>,
    pub new_sale_inventory: Account<'info, TokenAccount>,
}

#[derive(Accounts)]
pub struct BuyTani<'info> {
    #[account(
        mut,
        seeds = [PLATFORM_CONFIG_SEED],
        bump = platform_config.bump,
    )]
    pub platform_config: Account<'info, PlatformConfig>,
    #[account(mut)]
    pub buyer: Signer<'info>,
    #[account(
        mut,
        constraint = buyer_usdt_account.owner == buyer.key(),
        constraint = buyer_usdt_account.mint == platform_config.usdt_mint,
    )]
    pub buyer_usdt_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = buyer_tani_account.owner == buyer.key(),
        constraint = buyer_tani_account.mint == platform_config.tani_mint,
    )]
    pub buyer_tani_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = usdt_treasury.key() == platform_config.usdt_treasury,
    )]
    pub usdt_treasury: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = sale_inventory.key() == platform_config.sale_inventory,
    )]
    pub sale_inventory: Account<'info, TokenAccount>,
    #[account(seeds = [b"sale_authority"], bump)]
    /// CHECK: PDA authority untuk sale inventory
    pub sale_authority: AccountInfo<'info>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct BuyTaniReferred<'info> {
    #[account(
        mut,
        seeds = [PLATFORM_CONFIG_SEED],
        bump = platform_config.bump,
    )]
    pub platform_config: Account<'info, PlatformConfig>,
    #[account(mut)]
    pub buyer: Signer<'info>,
    #[account(
        mut,
        constraint = buyer_usdt_account.owner == buyer.key(),
        constraint = buyer_usdt_account.mint == platform_config.usdt_mint,
    )]
    pub buyer_usdt_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = buyer_tani_account.owner == buyer.key(),
        constraint = buyer_tani_account.mint == platform_config.tani_mint,
    )]
    pub buyer_tani_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = usdt_treasury.key() == platform_config.usdt_treasury,
    )]
    pub usdt_treasury: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = sale_inventory.key() == platform_config.sale_inventory,
    )]
    pub sale_inventory: Account<'info, TokenAccount>,
    /// USDT account milik referrer — menerima 5% bonus
    #[account(
        mut,
        constraint = referrer_usdt_account.mint == platform_config.usdt_mint,
        constraint = referrer_usdt_account.owner != buyer.key() @ Z4Error::SelfReferral,
    )]
    pub referrer_usdt_account: Account<'info, TokenAccount>,
    #[account(seeds = [b"sale_authority"], bump)]
    /// CHECK: PDA authority untuk sale inventory
    pub sale_authority: AccountInfo<'info>,
    pub token_program: Program<'info, Token>,
}
