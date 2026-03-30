use anchor_lang::prelude::*;

#[account]
pub struct PlatformConfig {
	pub authority: Pubkey,           // admin wallet
	pub tani_mint: Pubkey,           // $TANI token mint
	pub usdt_mint: Pubkey,           // USDT token mint
	pub sale_inventory: Pubkey,      // Sale Inventory token account
	pub usdt_treasury: Pubkey,       // USDT Treasury token account
	pub tani_treasury: Pubkey,       // TANI Allocation Treasury token account
	pub tani_per_usdt: u64,          // rate: berapa TANI per 1 USDT (dalam basis points)
	pub token_sale_active: bool,     // toggle token sale
	pub allocation_active: bool,     // toggle allocation
	pub total_tani_sold: u64,        // tracking total terjual
	pub total_tani_burned: u64,      // tracking total burned
	pub bump: u8,
}

impl PlatformConfig {
	pub const LEN: usize = 8   // discriminator
		+ 32   // authority
		+ 32   // tani_mint
		+ 32   // usdt_mint
		+ 32   // sale_inventory
		+ 32   // usdt_treasury
		+ 32   // tani_treasury
		+ 8    // tani_per_usdt
		+ 1    // token_sale_active
		+ 1    // allocation_active
		+ 8    // total_tani_sold
		+ 8    // total_tani_burned
		+ 1;   // bump
}

