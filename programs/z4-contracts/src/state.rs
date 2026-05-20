use anchor_lang::prelude::*;

#[account]
pub struct PlatformConfig {
    pub authority: Pubkey,
    pub tani_mint: Pubkey,
    pub usdt_mint: Pubkey,
    pub sale_inventory: Pubkey,
    pub usdt_treasury: Pubkey,
    pub tani_per_usdt: u64,
    pub token_sale_active: bool,
    pub total_tani_sold: u64,
    pub bump: u8,
}

impl PlatformConfig {
    pub const LEN: usize = 8 + 32 + 32 + 32 + 32 + 32 + 8 + 1 + 8 + 1;
}
