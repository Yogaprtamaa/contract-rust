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

    // Batch
    #[msg("Batch tidak dalam status open")]
    BatchNotOpen,
    #[msg("Batch sudah berakhir")]
    BatchExpired,
    #[msg("Batch belum berakhir")]
    BatchNotEnded,
    #[msg("Jumlah NFT tidak valid")]
    InvalidNftQuantity,
    #[msg("Kapasitas batch tidak cukup")]
    InsufficientBatchCapacity,
    #[msg("Batch belum mencapai target")]
    BatchTargetNotMet,
    #[msg("Batch sudah gagal")]
    BatchFailed,
    #[msg("Batch belum gagal")]
    BatchNotFailed,
    #[msg("Receipt sudah diklaim")]
    ReceiptAlreadyClaimed,
    #[msg("Batch sudah di-finalize")]
    BatchAlreadyFinalized,

    // Allocation
    #[msg("Saldo $TANI tidak cukup")]
    InsufficientTani,
    #[msg("Allocation sedang tidak aktif")]
    AllocationInactive,

    // General
    #[msg("Overflow kalkulasi")]
    Overflow,
    #[msg("Tidak authorized")]
    Unauthorized,
}
