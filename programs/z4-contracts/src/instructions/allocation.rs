use anchor_lang::prelude::*;
use anchor_spl::token::{self, Burn, Token, TokenAccount, Transfer};
use crate::errors::Z4Error;
use crate::events::PlotAllocated;
use crate::state::PlatformConfig;

#[derive(Accounts)]
#[instruction(plot_id: String, nft_id: String)]
pub struct AllocatePlot<'info> {
	#[account(
		mut,
		seeds = [b"platform_config"],
		bump = platform_config.bump,
	)]
	pub platform_config: Account<'info, PlatformConfig>,

	// User accounts
	#[account(mut)]
	pub user: Signer<'info>,

	#[account(
		mut,
		constraint = user_tani_account.owner == user.key(),
		constraint = user_tani_account.mint == platform_config.tani_mint,
	)]
	pub user_tani_account: Account<'info, TokenAccount>,

	// Platform accounts
	#[account(
		mut,
		constraint = tani_treasury.key() == platform_config.tani_treasury,
	)]
	pub tani_treasury: Account<'info, TokenAccount>,

	/// CHECK: TANI mint untuk burn
	#[account(
		mut,
		constraint = tani_mint.key() == platform_config.tani_mint,
	)]
	pub tani_mint: AccountInfo<'info>,

	pub token_program: Program<'info, Token>,
}

pub fn handler(
	ctx: Context<AllocatePlot>,
	plot_id: String,
	nft_id: String,
	tani_amount: u64,
) -> Result<()> {
	let config = &mut ctx.accounts.platform_config;

	// Validasi
	require!(config.allocation_active, Z4Error::AllocationInactive);
	require!(tani_amount > 0, Z4Error::InvalidAllocationAmount);
	require!(
		ctx.accounts.user_tani_account.amount >= tani_amount,
		Z4Error::InsufficientTani
	);

	// Hitung routing 70/30
	// 70% ke TANI Allocation Treasury
	// 30% di-burn via SPL Token burn instruction
	let treasury_amount = tani_amount
		.checked_mul(70)
		.ok_or(Z4Error::Overflow)?
		.checked_div(100)
		.ok_or(Z4Error::Overflow)?;

	let burn_amount = tani_amount
		.checked_sub(treasury_amount)
		.ok_or(Z4Error::Overflow)?;

	// Step 1: Transfer 70% $TANI ke TANI Allocation Treasury
	let cpi_accounts_treasury = Transfer {
		from: ctx.accounts.user_tani_account.to_account_info(),
		to: ctx.accounts.tani_treasury.to_account_info(),
		authority: ctx.accounts.user.to_account_info(),
	};
	let cpi_ctx_treasury = CpiContext::new(
		ctx.accounts.token_program.to_account_info(),
		cpi_accounts_treasury,
	);
	token::transfer(cpi_ctx_treasury, treasury_amount)?;

	// Step 2: Burn 30% $TANI secara valid via SPL Token program
	// Ini benar-benar mengurangi total supply, bukan transfer ke dead wallet
	let cpi_accounts_burn = Burn {
		mint: ctx.accounts.tani_mint.to_account_info(),
		from: ctx.accounts.user_tani_account.to_account_info(),
		authority: ctx.accounts.user.to_account_info(),
	};
	let cpi_ctx_burn = CpiContext::new(
		ctx.accounts.token_program.to_account_info(),
		cpi_accounts_burn,
	);
	token::burn(cpi_ctx_burn, burn_amount)?;

	// Update stats
	config.total_tani_burned = config.total_tani_burned
		.checked_add(burn_amount)
		.ok_or(Z4Error::Overflow)?;

	// Emit event — backend listener akan pick up ini
	emit!(PlotAllocated {
		user: ctx.accounts.user.key(),
		plot_id: plot_id.clone(),
		tani_spent: tani_amount,
		treasury_amount,
		burn_amount,
		nft_id: nft_id.clone(),
		timestamp: Clock::get()?.unix_timestamp,
	});

	msg!(
		"Allocation: plot={} tani={} treasury={} burn={} nft={}",
		plot_id, tani_amount, treasury_amount, burn_amount, nft_id
	);
	Ok(())
}

