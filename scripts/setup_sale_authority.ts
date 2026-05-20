import * as anchor from "@coral-xyz/anchor";
import { PublicKey, Transaction } from "@solana/web3.js";
import {
    getAssociatedTokenAddress,
    createSetAuthorityInstruction,
    AuthorityType,
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

    const saleInventoryKeypairFile = fs.readFileSync(
        "/root/z4-wallets/sale-inventory.json", "utf-8"
    );
    const saleInventoryKeypair = anchor.web3.Keypair.fromSecretKey(
        Uint8Array.from(JSON.parse(saleInventoryKeypairFile))
    );

    const TANI_MINT = new PublicKey("82uRtk77equ3QPbRdkzU7Hu5XKWt5ryAB9nGP8djRwSD");
    const SALE_INVENTORY_WALLET = new PublicKey("9sanq7Ysku7ND2bidsHDFH6d36J7jt3fGZqo3LPBQ1U6");
    const PROGRAM_ID = new PublicKey("9AShqzX8Y1uHDHxuhUvd6ojTwFa2BiBKPKHMSBSmV8MJ");

    const [saleAuthority] = PublicKey.findProgramAddressSync(
        [Buffer.from("sale_authority")],
        PROGRAM_ID
    );

    const saleInventoryTokenAccount = await getAssociatedTokenAddress(
        TANI_MINT, SALE_INVENTORY_WALLET
    );

    console.log("Sale Inventory Token Account:", saleInventoryTokenAccount.toString());
    console.log("Sale Authority PDA:", saleAuthority.toString());
    console.log("Current Owner:", saleInventoryKeypair.publicKey.toString());
    console.log("\nTransfer authority ke PDA sale_authority...");

    const ix = createSetAuthorityInstruction(
        saleInventoryTokenAccount,
        saleInventoryKeypair.publicKey,
        AuthorityType.AccountOwner,
        saleAuthority,
    );

    const tx = new Transaction().add(ix);
    const sig = await anchor.web3.sendAndConfirmTransaction(
        connection, tx, [keypair, saleInventoryKeypair]
    );

    console.log("Authority berhasil dipindahkan!");
    console.log("Transaction:", sig);
    console.log("Explorer:", `https://explorer.solana.com/tx/${sig}?cluster=devnet`);
}

main().catch(console.error);
