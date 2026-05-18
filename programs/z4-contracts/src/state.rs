use anchor_lang::prelude::*;

#[account]
pub struct PlatformConfig {
    pub authority: Pubkey,
    pub tani_mint: Pubkey,
    pub usdt_mint: Pubkey,
    pub sale_inventory: Pubkey,
    pub usdt_treasury: Pubkey,
    pub tani_treasury: Pubkey,
    pub tani_per_usdt: u64,
    pub token_sale_active: bool,
    pub allocation_active: bool,
    pub total_tani_sold: u64,
    pub total_tani_burned: u64,
    pub bump: u8,
}

impl PlatformConfig {
    pub const LEN: usize = 8 + 32 + 32 + 32 + 32 + 32 + 32 + 8 + 1 + 1 + 8 + 8 + 1;
}

#[account]
pub struct BatchAccount {
    pub batch_id: [u8; 20],        // batch identifier
    pub authority: Pubkey,          // admin yang buat batch
    pub plot_id: [u8; 10],         // plot yang di-batch
    pub tani_per_nft: u64,         // harga per NFT dalam $TANI lamports
    pub total_nft: u64,            // total NFT tersedia
    pub target_nft: u64,           // target minimum (70%)
    pub filled_nft: u64,           // yang sudah terjual
    pub total_tani_collected: u64, // total $TANI di vault
    pub status: BatchStatus,       // open/success/failed/completed
    pub start_time: i64,
    pub end_time: i64,             // start + 30 hari
    pub bump: u8,
    pub vault_bump: u8,
}

impl BatchAccount {
    pub const LEN: usize = 8 + 20 + 32 + 10 + 8 + 8 + 8 + 8 + 8 + 1 + 8 + 8 + 1 + 1 + 64;
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq)]
pub enum BatchStatus {
    Open,
    Success,
    Failed,
    Completed,
}

#[account]
pub struct AllocationReceipt {
    pub batch_id: [u8; 20],        // batch yang diikuti
    pub owner: Pubkey,             // wallet user
    pub nft_quantity: u64,         // berapa NFT yang dibeli
    pub tani_amount: u64,          // total $TANI yang disetor
    pub status: ReceiptStatus,     // pending/minted/refunded
    pub created_at: i64,
    pub bump: u8,
}

impl AllocationReceipt {
    pub const LEN: usize = 8 + 20 + 32 + 8 + 8 + 1 + 8 + 1 + 64;
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq)]
pub enum ReceiptStatus {
    Pending,
    Minted,
    Refunded,
}
