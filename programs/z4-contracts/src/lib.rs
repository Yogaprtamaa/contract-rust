use anchor_lang::prelude::*;

pub mod errors;
pub mod events;
pub mod instructions;
pub mod state;

pub use instructions::allocation::AllocatePlot;
pub use instructions::token_sale::BuyTani;
use instructions::allocation::__client_accounts_allocate_plot;
use instructions::token_sale::__client_accounts_buy_tani;
use instructions::{allocation, token_sale};
use state::PlatformConfig;

declare_id!("8tUz3PDatBckE2FPAmFx4UUDV59SustzdmcwS7sLpbi1");

#[program]
pub mod z4_contracts {
	use super::*;

	// ─── Initialize Platform ───────────────────────────────────────────────

	pub fn initialize(ctx: Context<Initialize>, tani_per_usdt: u64) -> Result<()> {
		let config = &mut ctx.accounts.platform_config;
		config.authority = ctx.accounts.authority.key();
		config.tani_mint = ctx.accounts.tani_mint.key();
		config.usdt_mint = ctx.accounts.usdt_mint.key();
		config.sale_inventory = ctx.accounts.sale_inventory.key();
		config.usdt_treasury = ctx.accounts.usdt_treasury.key();
		config.tani_treasury = ctx.accounts.tani_treasury.key();
		config.tani_per_usdt = tani_per_usdt;
		config.token_sale_active = true;
		config.allocation_active = true;
		config.total_tani_sold = 0;
		config.total_tani_burned = 0;
		config.bump = ctx.bumps.platform_config;

		msg!("Z4 Platform initialized. Rate: {} TANI per USDT", tani_per_usdt);
		Ok(())
	}

	// ─── Admin: Update Rate ────────────────────────────────────────────────

	pub fn update_rate(ctx: Context<AdminAction>, new_rate: u64) -> Result<()> {
		require!(new_rate > 0, errors::Z4Error::InvalidRate);
		ctx.accounts.platform_config.tani_per_usdt = new_rate;
		msg!("Rate diupdate: {} TANI per USDT", new_rate);
		Ok(())
	}

	// ─── Admin: Toggle Token Sale ──────────────────────────────────────────

	pub fn toggle_token_sale(ctx: Context<AdminAction>, active: bool) -> Result<()> {
		ctx.accounts.platform_config.token_sale_active = active;
		msg!("Token Sale: {}", if active { "AKTIF" } else { "NONAKTIF" });
		Ok(())
	}

	// ─── Admin: Toggle Allocation ──────────────────────────────────────────

	pub fn toggle_allocation(ctx: Context<AdminAction>, active: bool) -> Result<()> {
		ctx.accounts.platform_config.allocation_active = active;
		msg!("Allocation: {}", if active { "AKTIF" } else { "NONAKTIF" });
		Ok(())
	}

	// ─── Flow 1: Token Sale (USDT → $TANI) ────────────────────────────────

	pub fn buy_tani(ctx: Context<BuyTani>, usdt_amount: u64) -> Result<()> {
		token_sale::handler(ctx, usdt_amount)
	}

	// ─── Flow 2: Allocation ($TANI → 70% Treasury + 30% Burn + NFT) ───────

	pub fn allocate_plot(
		ctx: Context<AllocatePlot>,
		plot_id: String,
		nft_id: String,
		tani_amount: u64,
	) -> Result<()> {
		allocation::handler(ctx, plot_id, nft_id, tani_amount)
	}
}

// ─── Contexts ──────────────────────────────────────────────────────────────

#[derive(Accounts)]
pub struct Initialize<'info> {
	#[account(
		init,
		payer = authority,
		space = PlatformConfig::LEN,
		seeds = [b"platform_config"],
		bump,
	)]
	pub platform_config: Account<'info, PlatformConfig>,

	#[account(mut)]
	pub authority: Signer<'info>,

	/// CHECK: TANI mint address
	pub tani_mint: AccountInfo<'info>,

	/// CHECK: USDT mint address
	pub usdt_mint: AccountInfo<'info>,

	/// CHECK: Sale Inventory token account
	pub sale_inventory: AccountInfo<'info>,

	/// CHECK: USDT Treasury token account
	pub usdt_treasury: AccountInfo<'info>,

	/// CHECK: TANI Allocation Treasury token account
	pub tani_treasury: AccountInfo<'info>,

	pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct AdminAction<'info> {
	#[account(
		mut,
		seeds = [b"platform_config"],
		bump = platform_config.bump,
		has_one = authority,
	)]
	pub platform_config: Account<'info, PlatformConfig>,

	pub authority: Signer<'info>,
}

