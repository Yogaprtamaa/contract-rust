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
 * Example: 1 USD = 17.674 IDR, TANI = Rp 10.000
 *   → 1 USDT = 1.7674 TANI → store 17 on-chain (floor, buyer pays Rp 10.396/TANI)
 *
 * Only calls update_rate if the rounded integer changes (dead zone = 1 unit = 0.1 TANI/USDT).
 * This avoids burning SOL for sub-0.1 fluctuations.
 */

import * as anchor from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
import * as dotenv from "dotenv";
import fs from "fs";

dotenv.config();

// ── Config ──────────────────────────────────────────────────────────────────
const TANI_PRICE_IDR = 10_000;   // fixed IDR price of 1 TANI
const KEYPAIR_PATH   = process.env.KEYPAIR_PATH   ?? "/root/.config/solana/id.json";
const RPC_URL        = process.env.SOLANA_RPC_URL ?? "https://api.devnet.solana.com";

if (!process.env.SOLANA_RPC_URL) {
    console.warn("⚠️  SOLANA_RPC_URL not set in .env — falling back to public devnet RPC");
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

    // 2. Connect to devnet
    const keypairFile = fs.readFileSync(KEYPAIR_PATH, "utf-8");
    const keypair     = anchor.web3.Keypair.fromSecretKey(Uint8Array.from(JSON.parse(keypairFile)));
    const wallet      = new anchor.Wallet(keypair);
    const connection  = new anchor.web3.Connection(RPC_URL, "confirmed");
    const provider    = new anchor.AnchorProvider(connection, wallet, { commitment: "confirmed" });
    anchor.setProvider(provider);

    const idl     = JSON.parse(fs.readFileSync("./target/idl/z4_contracts.json", "utf-8"));
    const program = new anchor.Program(idl, provider);

    const [platformConfig] = PublicKey.findProgramAddressSync(
        [Buffer.from("platform_config")],
        program.programId
    );

    // 3. Read current on-chain rate
    const cfg         = await (program.account as any).platformConfig.fetch(platformConfig);
    const currentRate = cfg.taniPerUsdt.toNumber();
    console.log(`Current on-chain : ${currentRate} (= ${(currentRate / 10).toFixed(1)} TANI/USDT)`);

    // 4. Skip if unchanged
    if (newRate === currentRate) {
        console.log("✓ Rate unchanged — no update needed.\n");
        return;
    }

    const direction = newRate > currentRate ? "▲" : "▼";
    console.log(`\n${direction} Rate changed: ${currentRate} → ${newRate}. Updating on-chain…`);

    // 5. Send update_rate tx
    const tx = await (program.methods as any)
        .updateRate(new anchor.BN(newRate))
        .accounts({ platformConfig, authority: keypair.publicKey })
        .signers([keypair])
        .rpc();

    console.log(`✓ Updated!  tx: ${tx}`);
    console.log(`  Explorer : https://explorer.solana.com/tx/${tx}?cluster=devnet\n`);
}

main().catch((err) => {
    console.error("auto_update_rate failed:", err);
    process.exit(1);
});
