import * as anchor from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
import fs from "fs";

async function main() {
    const connection = new anchor.web3.Connection(
        "https://api.devnet.solana.com",
        "confirmed"
    );

    const keypairFile = fs.readFileSync("/root/.config/solana/id.json", "utf-8");
    const keypair = anchor.web3.Keypair.fromSecretKey(
        Uint8Array.from(JSON.parse(keypairFile))
    );

    const wallet = new anchor.Wallet(keypair);
    const provider = new anchor.AnchorProvider(connection, wallet, {
        commitment: "confirmed",
    });
    anchor.setProvider(provider);

    const idl = JSON.parse(
        fs.readFileSync("./target/idl/z4_contracts.json", "utf-8")
    );
    const program = new anchor.Program(idl, provider);

    const [platformConfig] = PublicKey.findProgramAddressSync(
        [Buffer.from("platform_config")],
        program.programId
    );

    // Rate disimpan dalam skala ×10
    // 1 TANI = Rp 10.000, 1 USDT = Rp 17.000
    // → 1 USDT = 1.7 TANI → simpan sebagai 17
    const NEW_RATE = new anchor.BN(17);

    // Baca rate sebelumnya
    const configBefore = await (program.account as any).platformConfig.fetch(platformConfig);
    console.log("Rate sekarang :", configBefore.taniPerUsdt.toString());
    console.log("Rate baru     :", NEW_RATE.toString(), "(= 1.7 TANI/USDT = Rp 10.000/TANI)");
    console.log("");

    const tx = await program.methods
        .updateRate(NEW_RATE)
        .accounts({
            platformConfig,
            authority: keypair.publicKey,
        })
        .signers([keypair])
        .rpc();

    console.log("Rate berhasil diupdate!");
    console.log("Transaction:", tx);
    console.log("Explorer   :", `https://explorer.solana.com/tx/${tx}?cluster=devnet`);

    // Konfirmasi
    const configAfter = await (program.account as any).platformConfig.fetch(platformConfig);
    console.log("\nKonfirmasi rate on-chain:", configAfter.taniPerUsdt.toString(), "✓");
}

main().catch(console.error);
