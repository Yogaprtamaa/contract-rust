use anchor_lang::prelude::*;

#[error_code]
pub enum Z4Error {
	#[msg("Sale inventory tidak cukup stok $TANI")]
	InsufficientInventory,
	#[msg("Jumlah USDT harus lebih dari 0")]
	InvalidUsdtAmount,
	#[msg("Rate tidak valid")]
	InvalidRate,
	#[msg("Plot tidak tersedia untuk alokasi")]
	PlotNotAvailable,
	#[msg("Saldo $TANI tidak cukup")]
	InsufficientTani,
	#[msg("Jumlah alokasi tidak valid")]
	InvalidAllocationAmount,
	#[msg("Overflow kalkulasi")]
	Overflow,
	#[msg("Wallet tidak authorized")]
	Unauthorized,
	#[msg("Token Sale sedang tidak aktif")]
	TokenSaleInactive,
	#[msg("Allocation sedang tidak aktif")]
	AllocationInactive,
}

