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
**Network:** ter-deploy di **MAINNET dan devnet** dengan program ID yang sama.

> ⚠️ Karena program ID identik di dua network, `SOLANA_RPC_URL` sendirian yang menentukan
> kena network mana. Angka devnet dan mainnet BERBEDA JAUH — jangan pernah mengutip angka
> tanpa memastikan network-nya dulu. `be-rust/.env.local` menunjuk ke Helius **mainnet**.

### PDAs
```
platform_config → seeds: [b"platform_config"]
sale_authority  → seeds: [b"sale_authority"]
```

### Instructions
- `buy_tani(usdt_amount: u64)` — tanpa referral
- `buy_tani_referred(usdt_amount: u64)` — dengan referral, 5% ke referrer USDT ATA

### Supply & Inventory — MAINNET (verifikasi on-chain 2026-07-21)
- TANI mint: `GB2omz7CtjRGrKvbTUt1vptG17KMH1bWnP8iBjxWDqNt` — **210.000.000**, **decimals 9**
- Sale inventory: `CTUsQEocpVtezBssyLmovsvh6xkKpfq6a5Sz1JQrQdoU` — **1.599.989 TANI** (0,76%)
- `total_tani_sold`: 10,71 TANI · `sale_active`: **true** (jualan hidup, USDT asli)
- Sebaran 210jt: 49,24% `CPzjVHuzt…` (= `TANI_TREASURY_WALLET`) · 30% `3wKz2gLEN…`
  · 10% `FVnjQsjhs…` · 10% `55tYqQzQz…` · 0,76% sale_authority PDA
- ⚠️ Keempat holder besar itu **tidak punya keypair di `z4-backup`** — custody 99,2% supply
  tidak terlacak dari file lokal.
- ⚠️ `Seedrym_PRD_MVP_v1.md` nulis "decimals 6 · $0,10" — dua-duanya salah (asli: 9, dan
  harga ditentukan `tani_per_usdt`). Formula `× 100` di `token_sale.rs` mengasumsikan
  decimals 9; jangan "perbaiki" jadi 6, itu bakal salah mint 1000×.

Devnet punya mint terpisah (`82uRtk…`, 219.999.880, dec 9) dengan inventory ~10jt — angka
devnet TIDAK berlaku untuk keputusan bisnis.

### Rate
- Mainnet saat ini: `tani_per_usdt = 53` (5,3 TANI/USDT) — diubah dari 17 pada 2026-07-21,
  tx `2oAn8GxpZch9spiyWXBxLvfeCb3yQyUemd9jHhCmHV9yADuZjkK98XMW3geCbjU9e8htzZumM1cCG4CocN61UCQn`
- Formula: `tani_raw = usdt_raw × tani_per_usdt × 100`
- Harga IDR dikunci di `scripts/auto_update_rate.ts` → `TANI_PRICE_IDR` (sekarang 3.333).
  Ubah di sana, bukan di rate. Rate diturunkan dari kurs USDT/IDR live.
- Target Rp 3.333 → rate **53** pada kurs ~Rp 17.900.

### IDL — hati-hati, ada dua
- ✅ `z4clone/src/types/z4.ts` (`export const IDL`) — address `9AShqzX8…`, punya `updateRate`
- ❌ `z4clone/src/idl/z4.json` — address `EEtj35um…`, **program lain**, tanpa `updateRate`.
  Memakainya bikin PDA salah dan error menyesatkan "account does not exist".

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
| Sale Inventory (ATA) | `EMZHThU4jweMEKPSpoUpDSe2CCDJDr3TMyV8NdVYDbcp` |
| USDT Treasury | `JDxqA2XymPzZM7rRfkmDLUEmfctY98ZqTUS4NSNLmTg3` |

> Dua alamat di atas diverifikasi dari `platform_config` on-chain (2026-07-21).
> Nilai lama di doc ini (`9sanq7…`, `87czwM…`) SALAH — `9sanq7…` bahkan tidak ada di devnet.
> `SALE_INVENTORY_WALLET` di `be-rust/.env.local` isinya `2GoDQnd…` = **sale_authority PDA**,
> bukan token account inventory. Namanya menyesatkan; saat ini tidak dipakai kode manapun.

---

## Build & Deploy

```bash
anchor build
anchor deploy --provider.cluster devnet

# Update rate — DRY-RUN default, tidak ada tx tanpa --execute
export SOLANA_RPC_URL=...   # penentu network! kosong = devnet publik
export PROGRAM_ID=9AShqzX8Y1uHDHxuhUvd6ojTwFa2BiBKPKHMSBSmV8MJ

npx tsx scripts/update_rate.ts --rate 53              # lihat dampak, aman
npx tsx scripts/update_rate.ts --rate 53 --execute    # kirim beneran
npx tsx scripts/auto_update_rate.ts                   # dry-run, pakai kurs live
npx tsx scripts/auto_update_rate.ts --selfcheck       # cek logika, tanpa jaringan
```

---

## Status Pekerjaan Selesai

- ✅ `buy_tani` + `buy_tani_referred` deployed devnet
- ⏳ Rate 51 (Rp 3.333/TANI) sudah di kode — **belum di-push on-chain**, jalankan `update_rate.ts`
- ✅ IDL di FE (`z4.ts`) sudah sinkron
- ✅ FE `buyTani.ts` handle dua path (dengan/tanpa referrer)
- ✅ BE listener parse `ReferralEvent`, insert `referral_earnings`
- ⏳ DB migration `20260520000001_add_referral.sql` perlu dijalankan di Supabase
