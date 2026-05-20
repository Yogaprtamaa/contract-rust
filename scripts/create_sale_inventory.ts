import * as anchor from "@coral-xyz/anchor";
import { PublicKey, Transaction } from "@solana/web3.js";
import {
    getAssociatedTokenAddress,
    createAssociatedTokenAccountInstruction,
} from "@solana/spl-token";
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

    const PROGRAM_ID = new PublicKey("9AShqzX8Y1uHDHxuhUvd6ojTwFa2BiBKPKHMSBSmV8MJ");
    const TANI_MINT  = new PublicKey("82uRtk77equ3QPbRdkzU7Hu5XKWt5ryAB9nGP8djRwSD");

    // Derive sale_authority PDA dari program baru
    const [saleAuthority] = PublicKey.findProgramAddressSync(
        [Buffer.from("sale_authority")],
        PROGRAM_ID
    );

    // ATA milik sale_authority PDA (allowOwnerOffCurve = true karena PDA)
    const saleInventoryATA = await getAssociatedTokenAddress(
        TANI_MINT,
        saleAuthority,
        true
    );

    console.log("Sale Authority PDA :", saleAuthority.toString());
    console.log("Sale Inventory ATA :", saleInventoryATA.toString());

    const exists = await connection.getAccountInfo(saleInventoryATA);
    if (exists) {
        console.log("Sudah ada, tidak perlu dibuat ulang.");
    } else {
        console.log("Membuat token account...");
        const ix = createAssociatedTokenAccountInstruction(
            keypair.publicKey,
            saleInventoryATA,
            saleAuthority,
            TANI_MINT,
        );
        const tx = new Transaction().add(ix);
        const sig = await anchor.web3.sendAndConfirmTransaction(connection, tx, [keypair]);
        console.log("Berhasil dibuat!");
        console.log("Tx    :", sig);
        console.log("Explorer:", `https://explorer.solana.com/tx/${sig}?cluster=devnet`);
    }

    console.log("\n========================================");
    console.log("Gunakan address ini sebagai SALE_INVENTORY:");
    console.log(saleInventoryATA.toString());
    console.log("========================================");
}

main().catch(console.error);
