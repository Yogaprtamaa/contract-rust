/**
 * auto_update_rate.ts
 *
 * Fetches live USD/IDR exchange rate and updates the on-chain tani_per_usdt
 * if the rate has shifted enough to matter.
 *
 * Formula:
 *   tani_per_usdt_real  = idr_per_usd / TANI_PRICE_IDR
 *   on_chain_value      = round(tani_per_usdt_real * 10)   ← ×10 scale used by contract
 *
 * Example: 1 USD = 17.674 IDR, TANI = Rp 10.000
 *   → 1 USDT = 1.7674 TANI → store 18 on-chain (rounds up)
 *
 * Only calls update_rate if the rounded integer changes (dead zone = 1 unit = 0.1 TANI/USDT).
 * This avoids burning SOL for sub-0.1 fluctuations.
 */

import * as anchor from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
import fs from "fs";

// ── Config ──────────────────────────────────────────────────────────────────
const TANI_PRICE_IDR = 10_000;   // fixed IDR price of 1 TANI
const KEYPAIR_PATH   = "/root/.config/solana/id.json";
const RPC_URL        = "https://api.devnet.solana.com";

// ── Fetch USD/IDR rate ───────────────────────────────────────────────────────
async function fetchUsdIdr(): Promise<number> {
    // frankfurter.app — free, no API key, maintained by ECB data
    const res  = await fetch("https://api.frankfurter.app/latest?from=USD&to=IDR");
    const data = await res.json() as { rates: { IDR: number } };
    if (!data?.rates?.IDR) throw new Error("IDR rate missing from response");
    return data.rates.IDR;
}

// ── Main ─────────────────────────────────────────────────────────────────────
async function main() {
    const now = new Date().toISOString();
    console.log(`\n[${now}] auto_update_rate starting…`);

    // 1. Fetch live rate
    const idrPerUsd = await fetchUsdIdr();
    const realRate  = idrPerUsd / TANI_PRICE_IDR;        // e.g. 1.7674
    const newRate   = Math.round(realRate * 10);          // e.g. 18  (×10 scale)
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
