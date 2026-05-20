# CLAUDE.md — z4-contracts

> Baca file ini dulu sebelum menyentuh kode apapun.

## Project ini adalah z4clone, BUKAN z4

| | Path |
|---|---|
| Smart Contract | `/root/clonez4/z4-contracts` (di sini) |
| Frontend | `C:\Users\reona\z4clone\z4-fe` → WSL: `/mnt/c/Users/reona/z4clone/z4-fe` |
| Backend | `C:\Users\reona\z4clone\be-rust` → WSL: `/mnt/c/Users/reona/z4clone/be-rust` |

**JANGAN sentuh `/root/z4/` — itu project lain yang tidak ada hubungannya.**

Acuan lengkap: `C:\Users\reona\z4clone\ACUAN.md`

---

## Program On-chain

**Program ID:** `9AShqzX8Y1uHDHxuhUvd6ojTwFa2BiBKPKHMSBSmV8MJ`
**Network:** Devnet

### PDAs
```
platform_config → seeds: [b"platform_config"]
sale_authority  → seeds: [b"sale_authority"]
```

### Instructions
- `buy_tani(usdt_amount: u64)` — tanpa referral
- `buy_tani_referred(usdt_amount: u64)` — dengan referral, 5% ke referrer USDT ATA

### Rate
- `tani_per_usdt = 17` (on-chain)
- Formula: `tani_raw = usdt_raw × 17 × 100`
- 1 TANI = Rp 10.000 (acuan 1 USDT = Rp 17.000)

### Events
```rust
TokenSaleEvent { wallet, usdt_amount, tani_received, rate, timestamp }
ReferralEvent  { buyer, referrer, usdt_amount, referral_bonus, tani_received, timestamp }
```
BE listener detect via: `log.contains("Referral:")`

---

## Token Addresses

| | Address |
|---|---|
| TANI | `82uRtk77equ3QPbRdkzU7Hu5XKWt5ryAB9nGP8djRwSD` |
| USDT | `5rj6AeTJYsHdVDF9DtKDazEtqm6zGe4Yr2orDB9Eydu5` |
| Sale Inventory | `9sanq7Ysku7ND2bidsHDFH6d36J7jt3fGZqo3LPBQ1U6` |
| USDT Treasury | `87czwMnrc8KzvLpa5QE5VyZ4SYs4SmB2X667rWw8yec1` |

---

## Build & Deploy

```bash
anchor build
anchor deploy --provider.cluster devnet

# Update rate (kalau perlu):
npx ts-node scripts/update_rate.ts
```

---

## Status Pekerjaan Selesai

- ✅ `buy_tani` + `buy_tani_referred` deployed devnet
- ✅ Rate 17 (Rp 10.000/TANI) sudah di-set on-chain
- ✅ IDL di FE (`z4.ts`) sudah sinkron
- ✅ FE `buyTani.ts` handle dua path (dengan/tanpa referrer)
- ✅ BE listener parse `ReferralEvent`, insert `referral_earnings`
- ⏳ DB migration `20260520000001_add_referral.sql` perlu dijalankan di Supabase
