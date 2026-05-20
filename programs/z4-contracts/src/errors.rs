use anchor_lang::prelude::*;

#[error_code]
pub enum Z4Error {
    // Token Sale
    #[msg("Sale inventory tidak cukup stok $TANI")]
    InsufficientInventory,
    #[msg("Jumlah USDT harus lebih dari 0")]
    InvalidUsdtAmount,
    #[msg("Rate tidak valid")]
    InvalidRate,
    #[msg("Token Sale sedang tidak aktif")]
    TokenSaleInactive,

    // Referral
    #[msg("Tidak bisa mereferral diri sendiri")]
    SelfReferral,

    // General
    #[msg("Overflow kalkulasi")]
    Overflow,
    #[msg("Tidak authorized")]
    Unauthorized,
}
