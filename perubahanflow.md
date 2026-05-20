# Z4 Smart Contract — Dev Plan (Devnet → Mainnet Ready)

> **Scope:** Token Sale ($TANI via USDT) only. Semua V2 cruft dihapus.
> **Target:** Lean, auditable, mainnet-safe contract.
> **Last updated:** 2026-05-19

---

## 🎯 Scope yang Dipertahankan

| Komponen | Status | Alasan |
|---|---|---|
| `instructions/token_sale.rs` | ✅ KEEP | Core V1 feature |
| `events.rs` → `TokenSaleEvent` | ✅ KEEP | Dibutuhkan BE listener sync |
| `state.rs` → `VaultPDA` | ✅ KEEP | Treasury wallet token sale |
| `errors.rs` | ✅ KEEP | Custom error codes |

---

## 🗑️ Yang Harus Dihapus

### 1. `instructions/allocation.rs` — HAPUS
**Kenapa:** NFT mint skip di V1. Instruction ini ada tapi ga dipake = dead code yang bisa dieksploit kalau ada celah validasi yang kelewat. Auditor juga charge per-instruction.

### 2. `instructions/batch.rs` — HAPUS
**Kenapa:** Batch finalize on-chain belum diaudit. Kalau ada bug di finalize logic → fund bisa lock permanen. V2 scope, jangan deploy mainnet sekarang.

### 3. `state.rs` → `AllocationRecord` — HAPUS
**Kenapa:** Hanya dipakai oleh allocation instruction. Kalau instruction-nya dihapus, state ini orphan. Tiap init AllocationRecord bayar ~0.002 SOL rent yang ga kepake.

### 4. `state.rs` → `BatchAccount` — HAPUS (atau strip)
**Kenapa:** Mirror DB, tapi kalau batch finalize on-chain dihapus, BatchAccount ga punya fungsi. Strip habis atau hapus total.

### 5. `events.rs` → `AllocationEvent` — HAPUS
**Kenapa:** Allocation instruction dihapus → event ini orphan.

### 6. `lib.rs.bak` & `lib.rs.bak2` — HAPUS
**Kenapa:** File backup dalam repo = masuk ke binary kalau ga di-exclude dengan benar. Confuse auditor, bloat size.

---

## 📁 Struktur Target Setelah Cleanup

```
programs/z4-contracts/src/
├── lib.rs              # hanya register token_sale
├── state.rs            # hanya VaultPDA
├── errors.rs           # hanya error untuk token_sale
├── events.rs           # hanya TokenSaleEvent
└── instructions/
    ├── mod.rs          # hanya export token_sale
    └── token_sale.rs   # USDT → $TANI, emit TokenSaleEvent
```

---

## 🔧 Token Sale Instruction — Spec

### Input Accounts
```rust
pub struct TokenSale<'info> {
    #[account(mut)]
    pub buyer: Signer<'info>,                      // user wallet
    pub usdt_mint: Account<'info, Mint>,           // USDT SPL token
    #[account(mut)]
    pub buyer_usdt_ata: Account<'info, TokenAccount>,  // buyer USDT ATA
    #[account(mut)]
    pub vault_usdt_ata: Account<'info, TokenAccount>,  // program USDT vault
    #[account(mut)]
    pub tani_mint: Account<'info, Mint>,           // $TANI mint (authority = program)
    #[account(mut)]
    pub buyer_tani_ata: Account<'info, TokenAccount>,  // buyer $TANI ATA
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}
```

### Logic
```
1. Validasi amount > 0
2. Transfer USDT dari buyer → vault (amount_usdt)
3. Mint $TANI ke buyer (amount_tani = amount_usdt × rate)
4. Emit TokenSaleEvent
```

### Event
```rust
#[event]
pub struct TokenSaleEvent {
    pub wallet: Pubkey,
    pub usdt_amount: u64,
    pub tani_received: u64,
    pub tx_hash: String,    // diisi FE setelah confirm
}
```

---

## ⚠️ Mainnet Disclaimer — Checklist Sebelum Deploy

- [ ] `anchor build` clean, zero warnings
- [ ] Unit test token_sale instruction: happy path + edge cases
- [ ] Test devnet E2E: wallet → USDT → $TANI received
- [ ] Tidak ada `unwrap()` atau `expect()` tanpa justifikasi
- [ ] Semua error state di-handle via `errors.rs`
- [ ] IDL di-copy ke FE: `w3fe/src/lib/idl/z4_contracts.json`
- [ ] Audit eksternal (opsional tapi **sangat disarankan** sebelum mainnet)
- [ ] Program upgrade authority di-set ke multisig (bukan single keypair)

---

## 💰 Estimasi Biaya Deploy Mainnet

> Kurs SOL: Rp 1.501.188 (2026-05-19)

| Item | SOL | IDR |
|---|---|---|
| Deploy program binary (~80-120KB setelah cleanup) | ~1.5–2 SOL | ~Rp 2.25jt – 3jt |
| IDL upload (`anchor idl init`) | ~0.1–0.2 SOL | ~Rp 150rb – 300rb |
| VaultPDA init (sekali) | ~0.002 SOL | ~Rp 3rb |
| **Total estimasi** | **~1.6–2.2 SOL** | **~Rp 2.4jt – 3.3jt** |

> ⚠️ Program upgrade setelah mainnet = bayar selisih size lagi. **Finalize contract sebelum deploy.**

---

## 🔄 Alur Kerja Dev

```
1. Hapus file V2 (allocation.rs, batch.rs, bak files)
2. Strip state.rs → hanya VaultPDA
3. Strip events.rs → hanya TokenSaleEvent
4. Update lib.rs → hanya register token_sale
5. anchor build → pastikan compile
6. anchor test → unit tests
7. anchor deploy --provider.cluster devnet
8. Copy IDL baru ke w3fe/src/lib/idl/
9. Test E2E di devnet
```