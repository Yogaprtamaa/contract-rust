use anchor_lang::prelude::*;

/// Emitted on every $TANI purchase via buy_tani instruction.
/// BE blockchain/listener.rs subscribes to this → inserts token_purchases table.
#[event]
pub struct TokenSaleEvent {
    pub wallet: Pubkey,
    pub usdt_amount: u64,
    pub tani_received: u64,
    pub rate: u64,      // tani_per_usdt at time of purchase
    pub timestamp: i64,
}

/// Emitted on every successful allocation (beli plot NFT).
/// BE blockchain/listener.rs subscribes to this → inserts/updates allocations table.
#[event]
pub struct AllocationEvent {
    pub wallet: Pubkey,
    pub batch_id: String,       // UUID canonical format (8-4-4-4-12)
    pub allocation_id: String,  // UUID canonical format (8-4-4-4-12)
    pub tani_spent: u64,
    pub timestamp: i64,
}

/// Emitted when a batch is finalized (success or failed).
#[event]
pub struct BatchFinalizedEvent {
    pub batch_id: String,
    pub result: String,     // "success" | "failed"
    pub funded_pct: u8,
    pub funded_tani: u64,
    pub timestamp: i64,
}

/// Emitted when a user claims a refund from a failed batch.
#[event]
pub struct RefundClaimedEvent {
    pub wallet: Pubkey,
    pub batch_id: String,
    pub allocation_id: String,
    pub tani_refunded: u64,
    pub timestamp: i64,
}
