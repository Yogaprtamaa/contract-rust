/**
 * update_rate.ts — ubah `tani_per_usdt` di platform_config.
 *
 * DRY-RUN secara default. Tidak ada transaksi terkirim tanpa flag `--execute`.
 * Program ini ter-deploy di devnet DAN mainnet dengan program ID yang sama, jadi
 * network ditentukan murni oleh SOLANA_RPC_URL — selalu baca banner sebelum lanjut.
 *
 *   npx tsx scripts/update_rate.ts                      # dry-run, lihat dampaknya
 *   npx tsx scripts/update_rate.ts --rate 53            # dry-run rate lain
 *   npx tsx scripts/update_rate.ts --rate 53 --execute  # kirim transaksi beneran
 *
 * Env:
 *   SOLANA_RPC_URL  RPC target (WAJIB diisi untuk mainnet)
 *   KEYPAIR_PATH    keypair authority
 *   TANI_PRICE_IDR  harga acuan buat kalkulasi tampilan (default 3333)
 */

import path from "path";
import fs from "fs";
import { loadDeps, loadIdl, assertIdlMatches, keypairPath, banner, namaNetwork, REPO } from "./_common.js";

function arg(name: string): string | undefined {
  const i = process.argv.indexOf(`--${name}`);
  return i >= 0 ? process.argv[i + 1] : undefined;
}

async function main() {
  const { anchor, web3, from: depsFrom } = loadDeps();
  const { PublicKey } = web3;

  const execute       = process.argv.includes("--execute");
  const RPC_URL       = process.env.SOLANA_RPC_URL ?? "https://api.devnet.solana.com";
  const KEYPAIR_PATH   = keypairPath();
  const TANI_PRICE_IDR = Number(process.env.TANI_PRICE_IDR ?? 3333);
  const NEW_RATE       = Number(arg("rate") ?? 53);

  if (!Number.isInteger(NEW_RATE) || NEW_RATE <= 0) {
    throw new Error(`--rate harus bilangan bulat positif (skala ×10), dapat: ${NEW_RATE}`);
  }

  const net = banner({
    mode: execute ? "EKSEKUSI — transaksi akan dikirim" : "DRY-RUN (tidak ada tx)",
    rpc: RPC_URL,
    depsFrom,
  });

  if (!process.env.SOLANA_RPC_URL) {
    console.warn("⚠️  SOLANA_RPC_URL tidak diset — jatuh ke devnet publik. Ini BUKAN mainnet.");
  }

  const { idl, from: idlFrom } = await loadIdl();
  console.log(`IDL          : ${idlFrom}`);
  console.log(`IDL address  : ${idl.address}`);
  assertIdlMatches(idl);   // program ID salah → PDA salah → error menyesatkan

  if (!fs.existsSync(KEYPAIR_PATH)) throw new Error(`Keypair tidak ada: ${KEYPAIR_PATH}`);
  const keypair = anchor.web3.Keypair.fromSecretKey(
    Uint8Array.from(JSON.parse(fs.readFileSync(KEYPAIR_PATH, "utf-8")))
  );

  const connection = new anchor.web3.Connection(RPC_URL, "confirmed");
  const provider   = new anchor.AnchorProvider(connection, new anchor.Wallet(keypair), {
    commitment: "confirmed",
  });
  anchor.setProvider(provider);
  const program = new anchor.Program(idl, provider);

  const [platformConfig] = PublicKey.findProgramAddressSync(
    [Buffer.from("platform_config")], program.programId
  );

  const cfg     = await (program.account as any).platformConfig.fetch(platformConfig);
  const current = cfg.taniPerUsdt.toNumber();

  const hargaDari = TANI_PRICE_IDR * (NEW_RATE / 10);   // kurs implisit di rate ini
  const show = (r: number) => `${r} (= ${(r / 10).toFixed(1)} TANI/USDT)`;

  console.log(`\nplatform_config : ${platformConfig.toBase58()}`);
  console.log(`signer          : ${keypair.publicKey.toBase58()}`);
  console.log(`config.authority: ${cfg.authority.toBase58()}`);
  console.log(`sale_active     : ${cfg.tokenSaleActive}`);
  console.log(`\nrate sekarang   : ${show(current)}`);
  console.log(`rate baru       : ${show(NEW_RATE)}`);
  console.log(`→ pada rate baru, 1 TANI = Rp ${TANI_PRICE_IDR.toLocaleString("id-ID")} `
            + `kalau kurs USDT/IDR = Rp ${hargaDari.toLocaleString("id-ID")}`);

  if (!keypair.publicKey.equals(cfg.authority)) {
    throw new Error(
      `Signer BUKAN authority. Transaksi pasti ditolak.\n` +
      `  signer   : ${keypair.publicKey.toBase58()}\n` +
      `  authority: ${cfg.authority.toBase58()}`
    );
  }

  if (current === NEW_RATE) {
    console.log("\n✓ Rate sudah sama persis — tidak ada yang perlu diubah.");
    return;
  }

  if (!execute) {
    console.log(`\n⏸  DRY-RUN selesai. Tidak ada yang berubah on-chain.`);
    console.log(`   Untuk benar-benar mengirim: tambahkan --execute`);
    if (net === "mainnet") {
      console.log(`   ⚠️  Target MAINNET — perubahan ini mengubah harga token asli untuk pembeli nyata.`);
    }
    return;
  }

  console.log(`\nMengirim update_rate…`);
  const tx = await (program.methods as any)
    .updateRate(new anchor.BN(NEW_RATE))
    .accounts({ platformConfig, authority: keypair.publicKey })
    .signers([keypair])
    .rpc();

  const cluster = net === "mainnet" ? "" : `?cluster=${net}`;
  console.log(`✓ Terkirim. tx: ${tx}`);
  console.log(`  Explorer   : https://explorer.solana.com/tx/${tx}${cluster}`);

  const after = await (program.account as any).platformConfig.fetch(platformConfig);
  console.log(`  Konfirmasi : ${show(after.taniPerUsdt.toNumber())}`);
}

main().catch((err) => {
  console.error("\n✗ update_rate gagal:", err.message ?? err);
  process.exit(1);
});
