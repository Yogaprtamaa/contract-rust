# Z4 On-Chain — Progress Tracker

> **Last updated:** 2026-05-19
> **Network:** Solana Devnet
> **Program ID:** `9AShqzX8Y1uHDHxuhUvd6ojTwFa2BiBKPKHMSBSmV8MJ`
> **Framework:** Anchor 0.32.1 · Rust 1.85.0 · Solana CLI 3.1.12

---

## Addresses Devnet (Aktif)

| Item | Address |
|---|---|
| Program ID | `9AShqzX8Y1uHDHxuhUvd6ojTwFa2BiBKPKHMSBSmV8MJ` |
| IDL Account | `2z4zHxMPAhKZKotKm4Lmm5M3DvEsMNx7xjZpwKQvyJen` |
| PlatformConfig PDA | `GmUCwW81RtomKK9f1CL2optqR5woEsDERkRvTa6tZatL` |
| Sale Authority PDA | `2GoDQndVeXtx9mz6fhBrHkjni8T5AT1ZodCnRvQot61D` |
| Sale Inventory ATA | `EMZHThU4jweMEKPSpoUpDSe2CCDJDr3TMyV8NdVYDbcp` |
| TANI Mint | `82uRtk77equ3QPbRdkzU7Hu5XKWt5ryAB9nGP8djRwSD` |
| USDT Mint | `5rj6AeTJYsHdVDF9DtKDazEtqm6zGe4Yr2orDB9Eydu5` |
| USDT Treasury ATA | `87czwMnrc8KzvLpa5QE5VyZ4SYs4SmB2X667rWw8yec1` |

---

## Status Keseluruhan

| Komponen | Status |
|---|---|
| Contract cleanup V1 (hapus batch/allocation) | ✅ Done |
| Compile & build clean | ✅ Done |
| Deploy devnet (program ID baru) | ✅ Done |
| Initialize PlatformConfig | ✅ Done |
| Sale inventory setup (ATA baru) | ✅ Done |
| TANI di-mint ke sale inventory | ✅ Done (10jt TANI) |
| `buy_tani` E2E test | 🔄 Pending |
| Copy IDL ke FE | 🔄 Pending |
| Update program ID di FE/BE | 🔄 Pending |

---

## Instruksi On-Chain (V1 — Production Scope)

### Admin
| Instruksi | Keterangan | Status |
|---|---|---|
| `initialize` | Setup platform: mint, treasury, rate | ✅ Done |
| `update_rate` | Update harga TANI per USDT | ✅ Done |
| `toggle_token_sale` | Aktif/nonaktif token sale | ✅ Done |
| `set_sale_inventory` | Update alamat sale inventory (admin only) | ✅ Done |

### User Flow
| Instruksi | Keterangan | Status |
|---|---|---|
| `buy_tani` | User beli $TANI dengan USDT → emit `TokenSaleEvent` | 🔄 Pending E2E test |

### Dihapus (V2 — tidak di-deploy)
| Instruksi | Alasan |
|---|---|
| `toggle_allocation` | V2 scope |
| `create_batch` | V2 scope, belum diaudit |
| `allocate_to_batch` | V2 scope |
| `finalize_batch` | V2 scope |
| `mint_nft_for_allocation` | V2 scope |
| `claim_refund` | V2 scope |

---

## Events (untuk BE Listener)

| Event | Trigger | Fields | BE Action |
|---|---|---|---|
| `TokenSaleEvent` | `buy_tani` berhasil | `wallet`, `usdt_amount`, `tani_received`, `rate`, `timestamp` | Insert `token_purchases` |

> ⚠️ Field lama `buyer` → sekarang `wallet`. Field `tani_amount` → sekarang `tani_received`. Update BE listener.

---

## State Accounts On-Chain

| Account | Seed | Fields |
|---|---|---|
| `PlatformConfig` | `["platform_config"]` | authority, tani_mint, usdt_mint, sale_inventory, usdt_treasury, tani_per_usdt, token_sale_active, total_tani_sold, bump |
| `SaleAuthority` | `["sale_authority"]` | PDA authority untuk sale inventory (tidak menyimpan data) |

> `BatchState`, `AllocationState`, `BatchVault` sudah dihapus dari V1.

---

## Toolchain

| Tool | Version |
|---|---|
| Rust | 1.85.0 (pinned di `rust-toolchain.toml`) |
| Anchor CLI | 0.32.1 |
| Solana CLI | 3.1.12 (Agave) |
| cargo-build-sbf | 3.1.12 / platform-tools v1.52 |
| Node | via ts-node |

---

## Changelog

### 2026-05-19
- **breaking:** Hapus semua V2 code (`allocation.rs`, `batch.rs`, `BatchState`, `AllocationState`, `BatchAccount`, semua batch events & errors)
- **refactor:** `lib.rs` direwrite lean — hanya token sale + admin instructions
- **refactor:** `state.rs` → hanya `PlatformConfig` (tanpa `tani_treasury`, `allocation_active`, `total_tani_burned`)
- **refactor:** `events.rs` → hanya `TokenSaleEvent`
- **refactor:** `errors.rs` → hanya 6 error token sale
- **fix:** Event field `buyer` → `wallet`, `tani_amount` → `tani_received`
- **fix:** `BuyTani` context dipindahkan ke `lib.rs` (Anchor macro limitation cross-module)
- **chore:** Program ID baru: `9AShqzX8Y1uHDHxuhUvd6ojTwFa2BiBKPKHMSBSmV8MJ`
- **chore:** `rust-toolchain.toml` → Rust 1.85.0 (fix hashbrown edition2024 compat)
- **chore:** `Anchor.toml` → cluster devnet, tambah `anchor_version = "0.32.1"`
- **feat:** Tambah `set_sale_inventory` instruction untuk migration
- **ops:** Sale inventory baru `EMZHThU4jwe...` (owned by new sale_authority PDA)
- **ops:** Mint 10jt TANI ke sale inventory baru

### 2026-05-18
- **feat:** Tambah event emission ke semua instruksi user-facing
- **feat:** Rewrite `events.rs` — events match `fullstack.md`
- **chore:** Hapus `lib.rs.bak` dan `lib.rs.bak2`

---

## Next Steps

1. Test E2E `buy_tani` dari FE atau script
2. Copy IDL ke FE: `target/idl/z4_contracts.json` → `w3fe/src/lib/idl/z4_contracts.json`
3. Update di FE/BE:
   - `PROGRAM_ID = 9AShqzX8Y1uHDHxuhUvd6ojTwFa2BiBKPKHMSBSmV8MJ`
   - `PLATFORM_CONFIG_PDA = GmUCwW81RtomKK9f1CL2optqR5woEsDERkRvTa6tZatL`
   - Event fields: `buyer` → `wallet`, `tani_amount` → `tani_received`
4. BE listener update: hapus subscriber `AllocationEvent`, `BatchFinalizedEvent`, `RefundClaimedEvent`
