import * as anchor from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
import { getAssociatedTokenAddress, createAssociatedTokenAccountInstruction, getAccount } from "@solana/spl-token";
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

    const TANI_MINT = new PublicKey("82uRtk77equ3QPbRdkzU7Hu5XKWt5ryAB9nGP8djRwSD");
    const USDT_MINT = new PublicKey("5rj6AeTJYsHdVDF9DtKDazEtqm6zGe4Yr2orDB9Eydu5");
    const SALE_INVENTORY_WALLET = new PublicKey("9sanq7Ysku7ND2bidsHDFH6d36J7jt3fGZqo3LPBQ1U6");
    const USDT_TREASURY_WALLET  = new PublicKey("87czwMnrc8KzvLpa5QE5VyZ4SYs4SmB2X667rWw8yec1");

    const [platformConfig] = PublicKey.findProgramAddressSync(
        [Buffer.from("platform_config")],
        program.programId
    );

    const [saleAuthority] = PublicKey.findProgramAddressSync(
        [Buffer.from("sale_authority")],
        program.programId
    );

    // Token accounts
    const buyerTaniAccount = await getAssociatedTokenAddress(TANI_MINT, keypair.publicKey);
    const buyerUsdtAccount = await getAssociatedTokenAddress(USDT_MINT, keypair.publicKey);
    const saleInventoryAccount = await getAssociatedTokenAddress(TANI_MINT, SALE_INVENTORY_WALLET);
    const usdtTreasuryAccount = await getAssociatedTokenAddress(USDT_MINT, USDT_TREASURY_WALLET);

    // Create ATAs if they don't exist yet
    const ixs: anchor.web3.TransactionInstruction[] = [];
    for (const { mint, ata } of [
        { mint: USDT_MINT, ata: buyerUsdtAccount },
        { mint: TANI_MINT, ata: buyerTaniAccount },
    ]) {
        try {
            await getAccount(connection, ata);
        } catch {
            ixs.push(
                createAssociatedTokenAccountInstruction(
                    keypair.publicKey,
                    ata,
                    keypair.publicKey,
                    mint
                )
            );
        }
    }
    if (ixs.length > 0) {
        const setupTx = new anchor.web3.Transaction().add(...ixs);
        const sig = await provider.sendAndConfirm(setupTx, [keypair]);
        console.log("Created missing ATAs:", sig);
    }

    // Cek saldo sebelum
    console.log("=== SEBELUM ===");
    try {
        const buyerTani = await getAccount(connection, buyerTaniAccount);
        console.log("Buyer TANI:", buyerTani.amount.toString());
    } catch { console.log("Buyer TANI account belum ada"); }

    try {
        const buyerUsdt = await getAccount(connection, buyerUsdtAccount);
        console.log("Buyer USDT:", buyerUsdt.amount.toString());
    } catch { console.log("Buyer USDT account belum ada atau kosong"); }

    // Beli 10 USDT worth of TANI (= 100 TANI karena rate 10:1)
    // USDT 6 desimal: 10 USDT = 10_000_000
    const usdtAmount = new anchor.BN(10_000_000);

    console.log("\nBeli 10 USDT → estimasi 100 TANI...");

    const tx = await program.methods
        .buyTani(usdtAmount)
        .accounts({
            platformConfig,
            buyer: keypair.publicKey,
            buyerUsdtAccount,
            buyerTaniAccount,
            usdtTreasury: usdtTreasuryAccount,
            saleInventory: saleInventoryAccount,
            saleAuthority,
            tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
        })
        .signers([keypair])
        .rpc();

    console.log("Transaction:", tx);
    console.log("Explorer:", `https://explorer.solana.com/tx/${tx}?cluster=devnet`);

    // Cek saldo sesudah
    console.log("\n=== SESUDAH ===");
    const buyerTaniAfter = await getAccount(connection, buyerTaniAccount);
    const buyerUsdtAfter = await getAccount(connection, buyerUsdtAccount);
    console.log("Buyer TANI:", buyerTaniAfter.amount.toString());
    console.log("Buyer USDT:", buyerUsdtAfter.amount.toString());

    console.log("\nToken Sale Flow berhasil!");
}

main().catch(console.error);
