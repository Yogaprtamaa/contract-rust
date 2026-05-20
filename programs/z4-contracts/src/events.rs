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

/// Emitted when a referral bonus is paid out.
/// referrer mendapat 5% dari usdt_amount pembelian.
#[event]
pub struct ReferralEvent {
    pub buyer: Pubkey,
    pub referrer: Pubkey,
    pub usdt_amount: u64,       // total pembelian buyer
    pub referral_bonus: u64,    // 5% dari usdt_amount → ke referrer
    pub tani_received: u64,
    pub timestamp: i64,
}
