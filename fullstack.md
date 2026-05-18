# Z4 — Full Stack Developer Context

> **Last updated:** 2026-05-18  
> **Scope V1:** USDT → swap $TANI (on-chain, DEX) → Beli Plot NFT → catat ke DB  
> **Out of scope V1:** NFT mint on-chain, harvest claims, tx verification

---

## 1. Repositories

| Repo | Stack | Path |
|---|---|---|
| Backend | Rust · Axum · sqlx · Supabase | `be-rust/` |
| Frontend | Next.js 14 (App Router) · TypeScript · Anchor | `w3fe/` |
| On-chain | Rust · Anchor · Solana | `z4-contracts/` |

---

## 2. User Flow (V1)

```
[User]
  │
  ├─1─ Connect wallet (Phantom / Solflare)
  │      FE: WalletProvider.tsx → useAuth.ts
  │      BE: POST /auth/connect → upsert users table
  │
  ├─2─ Get $TANI
  │      FE: /get-tani/page.tsx
  │      On-chain: token_sale instruction (z4-contracts)
  │      BE: POST /token-sale → catat token_purchases
  │
  └─3─ Beli Plot NFT
         FE: /plots/page.tsx → /plots/[plot_id]/page.tsx
         On-chain: allocation instruction (z4-contracts)
         BE: POST /allocations → catat allocations, update funded_tani
```

**Catatan penting:**
- Swap USDT → $TANI: full on-chain via DEX, backend hanya terima `tx_hash`
- NFT mint: **skip di V1**, allocation hanya dicatat di DB
- Tx verification Solana: **skip di V1**, backend trust client
- Auth: wallet address sebagai identifier, belum ada signature verification

---

## 3. Backend (`be-rust/`)

### Stack
- **Framework:** Axum
- **DB:** sqlx + Supabase (PostgreSQL)
- **Blockchain listener:** `blockchain/listener.rs` + `blockchain/solana_client.rs`

### Struktur
```
src/
├── main.rs                  # setup Axum router, DB pool, jalanin migrations
├── config.rs                # env vars (DATABASE_URL, SOLANA_RPC, dll)
├── errors.rs                # AppError enum → HTTP response
├── blockchain/
│   ├── listener.rs          # websocket listener on-chain events
│   └── solana_client.rs     # RPC calls ke Solana
├── db/                      # query functions (belum populate, siap diisi)
├── middleware/
│   └── auth.rs              # wallet address extractor dari header
├── models/                  # struct DB (sqlx FromRow)
│   ├── batch.rs
│   ├── allocation.rs
│   ├── plot.rs
│   ├── user.rs
│   ├── nft_record.rs
│   ├── purchase.rs
│   └── token_purchase.rs
└── routes/                  # Axum handler functions
    ├── auth.rs              # POST /auth/connect
    ├── batch.rs             # GET /batches, GET /batches/:id
    ├── plot.rs              # GET /plots, GET /plots/:id
    ├── allocation.rs        # POST /allocations, GET /allocations
    ├── purchase.rs          # POST /purchase
    ├── token_sale.rs        # POST /token-sale
    └── portfolio.rs         # GET /portfolio/:wallet
```

### API Endpoints (V1)

#### `GET /batches`
List batch dengan status `open`.
```json
// Response
[{
  "id": "uuid",
  "name": "Blok A — Banyuasin",
  "location": "Banyuasin, Sumatera Selatan",
  "commodity": "padi",
  "total_units": 1600,
  "funded_tani": 45000.0,
  "target_tani": 96000.0,
  "fill_percentage": 46.88,
  "min_fill_percentage": 70.0,
  "deadline_at": "2026-06-30T00:00:00Z",
  "status": "open"
}]
```

#### `GET /batches/:id`
Detail batch + unit tersisa.

#### `GET /plots?batch_id=:id`
List plot per batch.

#### `POST /allocations`
Beli Plot NFT — core endpoint V1.
```json
// Request
{
  "wallet_address": "SolPubkey...",
  "batch_id": "uuid",
  "plot_id": "plot-001",
  "quantity": 5,
  "tx_hash": "SolanaTxHash..."
}

// Response
{
  "allocation_id": "uuid",
  "wallet_address": "SolPubkey...",
  "batch_id": "uuid",
  "plot_id": "plot-001",
  "allocation_quantity": 5,
  "tani_spent": 300.0,
  "treasury_amount": 210.0,
  "burn_amount": 90.0,
  "status": "pending_batch",
  "created_at": "..."
}
```

**Logic flow `POST /allocations`:**
1. Validasi batch status = `open`
2. Validasi unit tersisa ≥ `quantity`
3. Hitung: `tani_spent = quantity × 60`, `treasury = 70%`, `burn = 30%`
4. Insert `allocations` (status: `pending_batch`)
5. Update `batches.funded_tani += tani_spent`
6. Return allocation record

#### `POST /auth/connect`
Upsert user saat connect wallet.
```json
// Request
{ "wallet_address": "SolPubkey..." }
```

#### `GET /portfolio/:wallet`
History allocation + status per user.

### Database Schema (ringkas)

```sql
-- Key tables untuk V1 scope
batches        → id, name, total_units, funded_tani, target_tani, status
plots          → id, batch_id, ...
users          → wallet_address (PK), created_at
allocations    → allocation_id, wallet_address, batch_id, plot_id,
                 quantity, tani_spent, treasury_amount, burn_amount,
                 tx_hash, status, created_at
token_purchases → record beli $TANI
```

**Allocation status flow:**
```
pending_batch → confirmed → minting → minted
                         ↘ failed → refunded
```

### Migration Files
```
migrations/
├── 20260329055999_init.sql
├── 20260329060001_create_users.sql
├── 20260329060002_create_plots.sql
├── 20260329060003_create_legal_references.sql
├── 20260329060004_create_token_purchases.sql
├── 20260329060005_create_allocations.sql
├── 20260329060006_create_nft_records.sql
├── 20260329060007_alter_batches_for_tani.sql       ← rekonstruksi
├── 20260329060008_alter_allocations_add_batch.sql  ← rekonstruksi
├── 20260329060009_create_batch_finalize_logs.sql   ← rekonstruksi
├── 20260329060010_create_harvests_claims.sql       ← placeholder
└── 20260401000100_alter_allocations_status_length.sql ← rekonstruksi
```

> ⚠️ Migration 60007–000100 hasil rekonstruksi. Original tidak pernah di-commit ke git.

---

## 4. Frontend (`w3fe/`)

### Stack
- **Framework:** Next.js 14 (App Router)
- **Wallet:** `@solana/wallet-adapter-react`
- **On-chain calls:** Anchor via `lib/anchor.ts` + IDL dari `lib/idl/z4_contracts.json`
- **API calls:** `lib/api.ts` → Axum backend

### Struktur
```
src/
├── app/
│   ├── page.tsx                  # landing / home
│   ├── layout.tsx                # root layout + WalletProvider
│   ├── get-tani/page.tsx         # halaman beli $TANI (token sale)
│   ├── plots/page.tsx            # list semua batch + plot
│   ├── plots/[plot_id]/          # detail plot + form beli
│   └── portfolio/page.tsx        # history allocation user
├── components/
│   ├── layout/Navbar.tsx         # navbar + wallet connect button
│   └── wallet/WalletProvider.tsx # Solana wallet adapter setup
├── hooks/
│   └── useAuth.ts                # wallet state + kirim ke BE
├── lib/
│   ├── api.ts                    # semua fetch ke BE (axios/fetch wrapper)
│   ├── anchor.ts                 # setup Anchor provider + program
│   ├── constants.ts              # PROGRAM_ID, API_URL, token address, dll
│   └── idl/z4_contracts.json     # IDL hasil anchor build
└── types/index.ts                # shared TypeScript types
```

### Key Files

**`lib/constants.ts`** — semua magic values di satu tempat:
```typescript
export const PROGRAM_ID = new PublicKey("...")
export const TANI_MINT = new PublicKey("...")
export const API_BASE_URL = process.env.NEXT_PUBLIC_API_URL
export const TANI_PER_NFT = 60
export const TREASURY_RATIO = 0.70
export const BURN_RATIO = 0.30
```

**`lib/api.ts`** — wrapper BE calls:
```typescript
// Contoh pattern
export const getBatches = () => fetch(`${API_BASE_URL}/batches`)
export const createAllocation = (body: AllocationRequest) =>
  fetch(`${API_BASE_URL}/allocations`, { method: 'POST', body: JSON.stringify(body) })
```

**`lib/anchor.ts`** — Anchor program instance:
```typescript
// Setup provider dari wallet adapter
// Load IDL → create Program instance
// Export functions: buyPlot(), tokenSale()
```

**`hooks/useAuth.ts`** — wallet connect flow:
```typescript
// 1. Detect wallet connected
// 2. POST /auth/connect ke BE
// 3. Store wallet_address di state/context
```

### Page Flow
```
/plots → list batches (GET /batches)
  └─ klik batch → /plots/[plot_id]
       └─ form: quantity input
            └─ klik Buy
                 ├─ call Anchor instruction: allocation
                 ├─ dapat tx_hash
                 └─ POST /allocations ke BE (dengan tx_hash)
```

---

## 5. On-chain (`z4-contracts/`)

### Stack
- **Framework:** Anchor (Rust)
- **Network:** Solana Devnet (→ Mainnet setelah audit)

### Struktur
```
programs/z4-contracts/src/
├── lib.rs              # entry point, register semua instructions
├── state.rs            # definisi account structs (on-chain state)
├── errors.rs           # custom error codes
├── events.rs           # events yang di-emit (untuk listener BE)
└── instructions/
    ├── mod.rs
    ├── allocation.rs   # logika beli plot NFT
    ├── batch.rs        # logika create/manage batch
    └── token_sale.rs   # logika beli $TANI
```

### Instructions

#### `token_sale`
User beli $TANI dengan USDT/SOL.
- Input: amount
- Output: transfer $TANI ke wallet user
- Event: emit `TokenSaleEvent`

#### `allocation`
User beli Plot NFT dengan $TANI.
- Input: batch_id, quantity
- Logic: transfer `quantity × 60 $TANI` → 70% treasury vault, 30% burn
- Output: record allocation on-chain
- Event: emit `AllocationEvent` (didengar oleh `blockchain/listener.rs` di BE)
- **V1:** NFT mint di-skip, hanya catat state

#### `batch`
Admin create/manage batch lahan.
- Create batch dengan parameter: total_units, target_tani, deadline
- Finalize batch setelah deadline

### State Accounts (state.rs)
```rust
// Key accounts yang ada on-chain
BatchAccount    → mirror dari batches table di DB
AllocationRecord → mirror dari allocations table di DB
VaultPDA        → treasury wallet per batch
```

### Events (events.rs)
BE `blockchain/listener.rs` subscribe ke events ini untuk sync DB:
```rust
AllocationEvent { wallet, batch_id, quantity, tani_spent, tx_hash }
TokenSaleEvent  { wallet, usdt_amount, tani_received, tx_hash }
```

### IDL
Build dengan `anchor build` → output ke `target/idl/z4_contracts.json`  
Copy ke FE: `w3fe/src/lib/idl/z4_contracts.json`

---

## 6. Inter-service Communication

```
FE ←──── REST API (JSON) ────→ BE
FE ←──── Anchor/RPC ─────────→ Solana
BE ←──── WebSocket/RPC ───────→ Solana (listener)
```

**Flow lengkap beli plot:**
```
FE: user klik Buy
  → FE: call Anchor instruction allocation
  → Solana: proses tx, emit AllocationEvent
  → FE: dapat tx_hash dari wallet
  → FE: POST /allocations ke BE (bawa tx_hash)
  → BE: insert DB, update funded_tani
  → BE (async): listener juga bisa catch AllocationEvent untuk double-sync
  → FE: tampilkan success
```

---

## 7. Environment Variables

### BE (`be-rust/.env`)
```env
DATABASE_URL=postgresql://...supabase.co/postgres
SOLANA_RPC_URL=https://api.devnet.solana.com
PROGRAM_ID=...
```

### FE (`w3fe/.env.local`)
```env
NEXT_PUBLIC_API_URL=http://localhost:3001
NEXT_PUBLIC_SOLANA_NETWORK=devnet
NEXT_PUBLIC_PROGRAM_ID=...
NEXT_PUBLIC_TANI_MINT=...
```

---

## 8. Business Rules (Immutable)

| Rule | Value |
|---|---|
| Harga 1 NFT | 60 $TANI = Rp 10.000 |
| Routing treasury | 70% dari setiap pembelian |
| Routing burn | 30% dari setiap pembelian (permanen) |
| Minimum batch fill | 70% sebelum bisa finalize |
| Harvest split | 50% holder / 50% Z4 |
| Hasil panen diklaim | dalam USDT (bukan $TANI) |
| Supply $TANI | 210.000.000 fixed |
| Siklus panen | ±90 hari (3x per tahun) |
| 1 NFT | = 1.25 m² lahan |

---

## 9. Known Issues & TODOs

| # | Issue | Komponen | Priority |
|---|---|---|---|
| 1 | Migration 60007–000100 hasil rekonstruksi | BE | HIGH |
| 2 | Auth belum ada signature verification | BE/FE | HIGH |
| 3 | Tx verification Solana di-skip | BE | HIGH (prod) |
| 4 | NFT mint belum diimplementasi | On-chain | V2 |
| 5 | `harvest_claims` hanya placeholder | BE/On-chain | V2 |
| 6 | `lib.rs.bak` & `lib.rs.bak2` belum dihapus | On-chain | LOW |
| 7 | `updated_at` batches belum ada auto-trigger | BE/DB | MEDIUM |