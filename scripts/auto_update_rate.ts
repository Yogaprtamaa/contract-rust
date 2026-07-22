/**
 * auto_update_rate.ts
 *
 * Fetches live USD/IDR exchange rate and updates the on-chain tani_per_usdt
 * if the rate has shifted enough to matter.
 *
 * Formula:
 *   tani_per_usdt_real  = idr_per_usd / TANI_PRICE_IDR
 *   on_chain_value      = floor(tani_per_usdt_real * 10)   ← ×10 scale, floor = never undersell
 *
 * Example: 1 USD = 17.674 IDR, TANI = Rp 3.333
 *   → 1 USDT = 5.3027 TANI → store 53 on-chain (floor, buyer pays Rp 3.335/TANI)
 *
 * Only calls update_rate if the rounded integer changes (dead zone = 1 unit = 0.1 TANI/USDT).
 * This avoids burning SOL for sub-0.1 fluctuations.
 */

import fs from "fs";
import { loadDeps, loadIdl, assertIdlMatches, keypairPath, banner, namaNetwork } from "./_common.js";

// anchor/web3/dotenv sengaja di-import lazy di dalam main() supaya `--selfcheck`
// bisa jalan tanpa node_modules terpasang.

// ── Config ──────────────────────────────────────────────────────────────────
const TANI_PRICE_IDR = 2_000;    // fixed IDR price of 1 TANI

// Dibaca di dalam main(), SETELAH dotenv.config() — kalau dibaca di level modul,
// nilai dari .env keburu terlewat karena dotenv sekarang di-load lazy.

// Toleransi kenaikan kurs sebelum bayar gas, dalam unit skala ×10.
// 2 = abaikan gerakan 1 unit (~1,9% kurs). Turunkan ke 1 kalau mau harga lebih ketat.
const MIN_UP_DELTA = 2;

/**
 * Sengaja ASIMETRIS — dua arah kurs punya konsekuensi yang tidak setara:
 *
 *  Kurs NAIK (newRate > current), update ditunda:
 *    on-chain rate ketinggalan di BAWAH → buyer dapat TANI lebih sedikit per USDT
 *    → buyer bayar DI ATAS TANI_PRICE_IDR. Platform tidak rugi → aman ditunda.
 *
 *  Kurs TURUN (newRate < current), update ditunda:
 *    on-chain rate ketinggalan di ATAS → buyer dapat TANI lebih banyak per USDT
 *    → buyer bayar DI BAWAH TANI_PRICE_IDR. Itu underselling inventory → JANGAN ditunda.
 *
 * Makanya penurunan selalu langsung dieksekusi, kenaikan boleh nunggu.
 *
 * ponytail: plafon MIN_UP_DELTA=2 → buyer bisa overpay sampai ~3,7% sebelum rate
 * dikoreksi. Kalau angka itu kegedean buat marketing/legal, set ke 1 (~1,9%).
 */
export function shouldUpdate(currentRate: number, newRate: number): boolean {
    if (newRate === currentRate) return false;
    if (newRate < currentRate)   return true;                    // turun → reaksi langsung
    return newRate - currentRate >= MIN_UP_DELTA;                // naik → toleransi jitter
}

// ── Fetch USD/IDR rate ───────────────────────────────────────────────────────

// Primary: Binance spot USDT/IDR — real-time trading price, no API key needed
async function fetchFromBinance(): Promise<number> {
    const res  = await fetch("https://api.binance.com/api/v3/ticker/price?symbol=USDTIDR");
    const data = await res.json() as { price: string };
    const rate = parseFloat(data.price);
    if (!rate || isNaN(rate)) throw new Error("Binance: invalid USDTIDR price");
    return rate;
}

// Fallback: Indodax (largest Indonesian exchange) — USDT/IDR spot
async function fetchFromIndodax(): Promise<number> {
    const res  = await fetch("https://indodax.com/api/usdt_idr/ticker");
    const data = await res.json() as { ticker: { last: string } };
    const rate = parseFloat(data.ticker.last);
    if (!rate || isNaN(rate)) throw new Error("Indodax: invalid last price");
    return rate;
}

async function fetchUsdIdr(): Promise<number> {
    try {
        const rate = await fetchFromBinance();
        console.log(`Rate source      : Binance (USDTIDR spot)`);
        return rate;
    } catch (err) {
        console.warn(`Binance failed (${err}), trying Indodax…`);
        const rate = await fetchFromIndodax();
        console.log(`Rate source      : Indodax (fallback)`);
        return rate;
    }
}

// ── Main ─────────────────────────────────────────────────────────────────────
async function main() {
    const { anchor, web3, from: depsFrom } = loadDeps();
    const { PublicKey } = web3;
    // dotenv cuma kenyamanan — kalau env sudah diekspor dari shell, tidak wajib ada.
    try { (await import("dotenv")).config(); } catch { /* lanjut tanpa .env */ }

    const execute      = process.argv.includes("--execute");
    const KEYPAIR_PATH = keypairPath();
    const RPC_URL      = process.env.SOLANA_RPC_URL ?? "https://api.devnet.solana.com";

    const net = banner({
        mode: execute ? "EKSEKUSI — transaksi akan dikirim" : "DRY-RUN (tidak ada tx)",
        rpc: RPC_URL,
        depsFrom,
    });
    if (!process.env.SOLANA_RPC_URL) {
        console.warn("⚠️  SOLANA_RPC_URL tidak diset — jatuh ke devnet publik. Ini BUKAN mainnet.");
    }

    const now = new Date().toISOString();
    console.log(`\n[${now}] auto_update_rate starting…`);

    // 1. Fetch live rate
    const idrPerUsd = await fetchUsdIdr();
    const realRate  = idrPerUsd / TANI_PRICE_IDR;        // e.g. 1.7674
    const newRate   = Math.floor(realRate * 10);          // e.g. 17  (floor = never undersell)
    console.log(`USD/IDR          : ${idrPerUsd.toFixed(2)}`);
    console.log(`TANI price (IDR) : Rp ${TANI_PRICE_IDR.toLocaleString()}`);
    console.log(`Real rate        : ${realRate.toFixed(4)} TANI/USDT`);
    console.log(`On-chain value   : ${newRate} (×10 scale = ${(newRate / 10).toFixed(1)} TANI/USDT)`);

    // 2. Sambung ke network sesuai SOLANA_RPC_URL
    if (!fs.existsSync(KEYPAIR_PATH)) throw new Error(`Keypair tidak ada: ${KEYPAIR_PATH}`);
    const keypairFile = fs.readFileSync(KEYPAIR_PATH, "utf-8");
    const keypair     = anchor.web3.Keypair.fromSecretKey(Uint8Array.from(JSON.parse(keypairFile)));
    const wallet      = new anchor.Wallet(keypair);
    const connection  = new anchor.web3.Connection(RPC_URL, "confirmed");
    const provider    = new anchor.AnchorProvider(connection, wallet, { commitment: "confirmed" });
    anchor.setProvider(provider);

    const { idl, from: idlFrom } = await loadIdl();
    assertIdlMatches(idl);
    console.log(`IDL              : ${idlFrom}`);
    const program = new anchor.Program(idl, provider);

    const [platformConfig] = PublicKey.findProgramAddressSync(
        [Buffer.from("platform_config")],
        program.programId
    );

    // 3. Read current on-chain rate
    const cfg         = await (program.account as any).platformConfig.fetch(platformConfig);
    const currentRate = cfg.taniPerUsdt.toNumber();
    console.log(`Current on-chain : ${currentRate} (= ${(currentRate / 10).toFixed(1)} TANI/USDT)`);

    // 4. Skip kalau gerakannya belum layak bayar gas
    if (!shouldUpdate(currentRate, newRate)) {
        const why = newRate === currentRate
            ? "rate unchanged"
            : `naik cuma ${newRate - currentRate} unit (< ${MIN_UP_DELTA}) — buyer bayar sedikit di atas harga, aman ditunda`;
        console.log(`✓ Skip update — ${why}.\n`);
        return;
    }

    const direction = newRate > currentRate ? "▲" : "▼";
    console.log(`\n${direction} Rate berubah: ${currentRate} → ${newRate}`);

    if (!keypair.publicKey.equals(cfg.authority)) {
        throw new Error(
            `Signer BUKAN authority — transaksi pasti ditolak.\n` +
            `  signer   : ${keypair.publicKey.toBase58()}\n` +
            `  authority: ${cfg.authority.toBase58()}`
        );
    }

    if (!execute) {
        console.log(`⏸  DRY-RUN — tidak ada yang dikirim. Tambahkan --execute untuk eksekusi.`);
        if (net === "mainnet") {
            console.log(`   ⚠️  Target MAINNET — ini mengubah harga token asli untuk pembeli nyata.`);
        }
        return;
    }

    // 5. Kirim update_rate
    const tx = await (program.methods as any)
        .updateRate(new anchor.BN(newRate))
        .accounts({ platformConfig, authority: keypair.publicKey })
        .signers([keypair])
        .rpc();

    const cluster = net === "mainnet" ? "" : `?cluster=${net}`;
    console.log(`✓ Terkirim. tx: ${tx}`);
    console.log(`  Explorer : https://explorer.solana.com/tx/${tx}${cluster}\n`);
}

// ── Self-check ───────────────────────────────────────────────────────────────
// Jalankan: npx ts-node scripts/auto_update_rate.ts --selfcheck  (tidak menyentuh chain)
function selfcheck() {
    const assert = (cond: boolean, msg: string) => {
        if (!cond) { console.error("FAIL:", msg); process.exit(1); }
    };

    assert(shouldUpdate(53, 53) === false, "rate sama → jangan update");

    // Kenaikan kecil ditoleransi, kenaikan besar tidak
    assert(shouldUpdate(53, 54) === false, "naik 1 unit → tunda (buyer overpay, aman)");
    assert(shouldUpdate(53, 55) === true,  "naik 2 unit → update");

    // Penurunan SELALU langsung, sekecil apapun — ini yang bikin platform rugi
    assert(shouldUpdate(53, 52) === true,  "turun 1 unit → wajib update (anti-underselling)");
    assert(shouldUpdate(53, 40) === true,  "turun besar → wajib update");

    // floor() tidak boleh pernah jual di bawah TANI_PRICE_IDR
    for (const idr of [16_000, 17_000, 17_900, 18_500, 20_000]) {
        const onChain = Math.floor((idr / TANI_PRICE_IDR) * 10);
        const harga   = idr / (onChain / 10);
        assert(harga >= TANI_PRICE_IDR, `kurs ${idr} → harga ${harga.toFixed(0)} di bawah floor`);
    }

    console.log("✓ selfcheck lolos");
}

if (process.argv.includes("--selfcheck")) {
    selfcheck();
} else {
    main().catch((err) => {
        console.error("auto_update_rate failed:", err);
        process.exit(1);
    });
}
