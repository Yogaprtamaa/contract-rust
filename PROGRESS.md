# Z4 On-Chain — Progress Tracker

> **Last updated:** 2026-05-18
> **Network:** Solana Devnet
> **Program ID:** `8tUz3PDatBckE2FPAmFx4UUDV59SustzdmcwS7sLpbi1`
> **Framework:** Anchor 0.32.1 · Rust

---

## Status Keseluruhan

| Komponen | Status |
|---|---|
| Platform initialize | Done |
| Token Sale (USDT → $TANI) | Done |
| Batch create/manage | Done |
| Allocation (beli plot NFT) | Done |
| Finalize batch (treasury + burn) | Done |
| Claim refund (batch gagal) | Done |
| NFT mint on-chain | Skipped (V2) |
| On-chain events emission | Done |

---

## Instruksi On-Chain

### Admin
| Instruksi | Keterangan | Status |
|---|---|---|
| `initialize` | Setup platform: mint, treasury, rate | Done |
| `update_rate` | Update harga TANI per USDT | Done |
| `toggle_token_sale` | Aktif/nonaktif token sale | Done |
| `toggle_allocation` | Aktif/nonaktif allocation | Done |

### User Flow
| Instruksi | Keterangan | Status |
|---|---|---|
| `buy_tani` | User beli $TANI dengan USDT → emit `TokenSaleEvent` | Done |
| `create_batch` | Admin buat batch lahan | Done |
| `allocate_to_batch` | User beli plot dengan $TANI → emit `AllocationEvent` | Done |
| `finalize_batch` | Admin finalize: 70% treasury + 30% burn → emit `BatchFinalizedEvent` | Done |
| `mint_nft_for_allocation` | Mint NFT per allocation (post-finalize) | Done (V2 ready) |
| `claim_refund` | User refund jika batch gagal → emit `RefundClaimedEvent` | Done |

---

## Events (untuk BE Listener)

BE `blockchain/listener.rs` subscribe ke events ini untuk sync DB:

| Event | Trigger | BE Action |
|---|---|---|
| `TokenSaleEvent` | `buy_tani` berhasil | Insert `token_purchases` |
| `AllocationEvent` | `allocate_to_batch` berhasil | Insert `allocations`, update `funded_tani` |
| `BatchFinalizedEvent` | `finalize_batch` berhasil | Update `batches.status` |
| `RefundClaimedEvent` | `claim_refund` berhasil | Update `allocations.status = refunded` |

---

## Business Rules (Immutable)

| Rule | Value |
|---|---|
| Harga 1 NFT | 60 $TANI = Rp 10.000 |
| Routing treasury | 70% dari setiap pembelian |
| Routing burn | 30% dari setiap pembelian (SPL burn, supply berkurang) |
| Minimum batch fill | 70% sebelum bisa finalize success |
| Burn mechanism | Real SPL burn (bukan transfer ke dead wallet) |

---

## State Accounts On-Chain

| Account | Seed | Keterangan |
|---|---|---|
| `PlatformConfig` | `["platform_config"]` | Config global platform |
| `BatchState` | `["batch_state", batch_uuid]` | State per batch |
| `BatchVault` | `["batch_vault", batch_uuid]` | Token vault per batch (PDA) |
| `AllocationState` | `["allocation", batch_uuid, allocation_uuid]` | Record per allocation user |
| `SaleAuthority` | `["sale_authority"]` | PDA authority inventory token sale |

---

## Changelog

### 2026-05-18
- **feat:** Tambah event emission ke semua instruksi user-facing (`buy_tani`, `allocate_to_batch`, `finalize_batch`, `claim_refund`)
- **feat:** Rewrite `events.rs` — events sekarang match nama di `fullstack.md` (`TokenSaleEvent`, `AllocationEvent`, `BatchFinalizedEvent`, `RefundClaimedEvent`)
- **chore:** Hapus `lib.rs.bak` dan `lib.rs.bak2`
- **fix:** `lib.rs` sekarang import `mod events` dan emit events untuk BE listener

---

## Known Issues & TODO

| # | Issue | Prioritas |
|---|---|---|
| 1 | `instructions/` folder (allocation.rs, batch.rs, token_sale.rs) adalah dead code — tidak diimport lib.rs | LOW (cleanup V1.1) |
| 2 | `state.rs` dan `errors.rs` juga orphaned — definisi duplikat dari lib.rs | LOW (cleanup V1.1) |
| 3 | Auth: belum ada signature verification | HIGH (prod) |
| 4 | Tx verification Solana di-skip di BE | HIGH (prod) |
| 5 | NFT mint on-chain belum diimplementasi | V2 |
| 6 | `harvest_claims` hanya placeholder | V2 |

---

## Next Steps

1. `anchor build` dari WSL environment (bukan Windows path) untuk generate IDL
2. Copy IDL ke FE: `w3fe/src/lib/idl/z4_contracts.json`
3. Implementasi `blockchain/listener.rs` di BE untuk subscribe events
4. Deploy ke Devnet: `anchor deploy --provider.cluster devnet`
