/**
 * Helper bersama untuk script rate (update_rate.ts, auto_update_rate.ts).
 *
 * Ada karena kedua script butuh resolusi yang sama persis dan gampang salah:
 * dependency, IDL yang benar, dan deteksi network. Salah satu saja meleset,
 * gejalanya jadi error yang menyesatkan (PDA salah → "account does not exist").
 */

import { createRequire } from "module";
import { fileURLToPath } from "url";
import path from "path";
import fs from "fs";

export const HERE = path.dirname(fileURLToPath(import.meta.url));
export const REPO = path.resolve(HERE, "../..");

/** contract-rust sering belum di-`npm install`; pinjam dependency dari z4clone. */
export function loadDeps() {
  const candidates = [
    path.join(HERE, "..", "package.json"),
    path.join(REPO, "z4clone", "package.json"),
  ];
  for (const c of candidates) {
    try {
      const req = createRequire(c);
      return { anchor: req("@coral-xyz/anchor"), web3: req("@solana/web3.js"), from: path.dirname(c) };
    } catch { /* kandidat berikutnya */ }
  }
  throw new Error("@coral-xyz/anchor tidak ketemu. `npm install` di contract-rust atau z4clone.");
}

/**
 * JANGAN pakai `z4clone/src/idl/z4.json` — itu IDL program LAIN
 * (address EEtj35um…, tidak punya instruksi update_rate).
 * Yang cocok dengan program ter-deploy 9AShqzX8… ada di `z4clone/src/types/z4.ts`.
 */
export async function loadIdl() {
  const json = path.join(HERE, "..", "target", "idl", "z4_contracts.json");
  if (fs.existsSync(json)) return { idl: JSON.parse(fs.readFileSync(json, "utf-8")), from: json };

  const ts = path.join(REPO, "z4clone", "src", "types", "z4.ts");
  if (fs.existsSync(ts)) {
    const mod = await import(ts);
    if (mod.IDL) return { idl: mod.IDL, from: ts };
  }
  throw new Error("IDL tidak ketemu. Jalankan `anchor build` di contract-rust dulu.");
}

export function assertIdlMatches(idl: any) {
  const expected = process.env.PROGRAM_ID;
  if (expected && idl.address !== expected) {
    throw new Error(
      `IDL bukan untuk program yang dituju.\n  IDL address : ${idl.address}\n  PROGRAM_ID  : ${expected}`
    );
  }
}

export type Net = "mainnet" | "devnet" | "testnet" | "tidak dikenal";

export function namaNetwork(url: string): Net {
  const u = url.toLowerCase();
  if (u.includes("devnet"))  return "devnet";
  if (u.includes("testnet")) return "testnet";
  if (u.includes("mainnet")) return "mainnet";
  return "tidak dikenal";
}

export function maskRpc(url: string) {
  return url.replace(/api-key=[^&]+/, "api-key=***");
}

/** Keypair authority. Default menunjuk ke lokasi nyata di repo ini, bukan path WSL lama. */
export function keypairPath() {
  return process.env.KEYPAIR_PATH ?? path.join(REPO, "z4-backup", "solana-config", "id.json");
}

export function banner(opts: { mode: string; rpc: string; depsFrom: string }) {
  const net = namaNetwork(opts.rpc);
  console.log("─".repeat(64));
  console.log(`  MODE     : ${opts.mode}`);
  console.log(`  NETWORK  : ${net.toUpperCase()}${net === "mainnet" ? "  ⚠️  UANG ASLI" : ""}`);
  console.log(`  RPC      : ${maskRpc(opts.rpc)}`);
  console.log(`  deps dari: ${opts.depsFrom}`);
  console.log("─".repeat(64));
  return net;
}
