use anchor_lang::prelude::*;

#[event]
pub struct TokenPurchased {
	pub buyer: Pubkey,
	pub usdt_amount: u64,
	pub tani_amount: u64,
	pub rate: u64,
	pub timestamp: i64,
}

#[event]
pub struct PlotAllocated {
	pub user: Pubkey,
	pub plot_id: String,
	pub tani_spent: u64,
	pub treasury_amount: u64,
	pub burn_amount: u64,
	pub nft_id: String,
	pub timestamp: i64,
}

