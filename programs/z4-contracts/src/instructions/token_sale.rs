use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};
use crate::errors::Z4Error;
use crate::events::TokenPurchased;
use crate::state::PlatformConfig;

#[derive(Accounts)]
pub struct BuyTani<'info> {
	#[account(
		mut,
		seeds = [b"platform_config"],
		bump = platform_config.bump,
	)]
	pub platform_config: Account<'info, PlatformConfig>,

	// Buyer accounts
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

	// Platform accounts
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

	/// CHECK: PDA authority untuk sale inventory
	#[account(
		seeds = [b"sale_authority"],
		bump,
	)]
	pub sale_authority: AccountInfo<'info>,

	pub token_program: Program<'info, Token>,
}

pub fn handler(ctx: Context<BuyTani>, usdt_amount: u64) -> Result<()> {
	let config = &mut ctx.accounts.platform_config;

	// Validasi
	require!(config.token_sale_active, Z4Error::TokenSaleInactive);
	require!(usdt_amount > 0, Z4Error::InvalidUsdtAmount);

	// Hitung TANI yang akan diterima
	// tani_per_usdt adalah rate dalam basis: 10 TANI per 1 USDT
	// USDT = 6 desimal, TANI = 9 desimal
	// Formula: tani_amount = usdt_amount * rate * 10^(9-6) = usdt_amount * rate * 1000
	let tani_amount = usdt_amount
		.checked_mul(config.tani_per_usdt)
		.ok_or(Z4Error::Overflow)?
		.checked_mul(1000)
		.ok_or(Z4Error::Overflow)?;

	// Validasi stok sale inventory
	require!(
		ctx.accounts.sale_inventory.amount >= tani_amount,
		Z4Error::InsufficientInventory
	);

	// Step 1: Transfer USDT dari buyer ke USDT Treasury
	let cpi_accounts_usdt = Transfer {
		from: ctx.accounts.buyer_usdt_account.to_account_info(),
		to: ctx.accounts.usdt_treasury.to_account_info(),
		authority: ctx.accounts.buyer.to_account_info(),
	};
	let cpi_ctx_usdt = CpiContext::new(
		ctx.accounts.token_program.to_account_info(),
		cpi_accounts_usdt,
	);
	token::transfer(cpi_ctx_usdt, usdt_amount)?;

	// Step 2: Transfer $TANI dari Sale Inventory ke buyer
	let sale_authority_bump = ctx.bumps.sale_authority;
	let seeds = &[b"sale_authority".as_ref(), &[sale_authority_bump]];
	let signer_seeds = &[&seeds[..]];

	let cpi_accounts_tani = Transfer {
		from: ctx.accounts.sale_inventory.to_account_info(),
		to: ctx.accounts.buyer_tani_account.to_account_info(),
		authority: ctx.accounts.sale_authority.to_account_info(),
	};
	let cpi_ctx_tani = CpiContext::new_with_signer(
		ctx.accounts.token_program.to_account_info(),
		cpi_accounts_tani,
		signer_seeds,
	);
	token::transfer(cpi_ctx_tani, tani_amount)?;

	// Update stats
	config.total_tani_sold = config.total_tani_sold
		.checked_add(tani_amount)
		.ok_or(Z4Error::Overflow)?;

	// Emit event
	emit!(TokenPurchased {
		buyer: ctx.accounts.buyer.key(),
		usdt_amount,
		tani_amount,
		rate: config.tani_per_usdt,
		timestamp: Clock::get()?.unix_timestamp,
	});

	msg!("Token Sale: {} USDT → {} TANI", usdt_amount, tani_amount);
	Ok(())
}

