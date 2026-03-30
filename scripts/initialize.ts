import * as anchor from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
import { getAssociatedTokenAddress } from "@solana/spl-token";
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

    // Token addresses
    const TANI_MINT = new PublicKey("82uRtk77equ3QPbRdkzU7Hu5XKWt5ryAB9nGP8djRwSD");
    const USDT_MINT = new PublicKey("5rj6AeTJYsHdVDF9DtKDazEtqm6zGe4Yr2orDB9Eydu5");

    // Wallet addresses
    const SALE_INVENTORY_WALLET = new PublicKey("9sanq7Ysku7ND2bidsHDFH6d36J7jt3fGZqo3LPBQ1U6");
    const USDT_TREASURY_WALLET  = new PublicKey("87czwMnrc8KzvLpa5QE5VyZ4SYs4SmB2X667rWw8yec1");
    const TANI_TREASURY_WALLET  = new PublicKey("8d56NGYEsWiQ3EiqVCF2WAQmTDEyZS5io8q1Q4Ui4XgG");

    // Token accounts
    const saleInventoryTokenAccount = await getAssociatedTokenAddress(
        TANI_MINT, SALE_INVENTORY_WALLET
    );
    const usdtTreasuryTokenAccount = await getAssociatedTokenAddress(
        USDT_MINT, USDT_TREASURY_WALLET
    );
    const taniTreasuryTokenAccount = await getAssociatedTokenAddress(
        TANI_MINT, TANI_TREASURY_WALLET
    );

    // Derive PlatformConfig PDA
    const [platformConfig] = PublicKey.findProgramAddressSync(
        [Buffer.from("platform_config")],
        program.programId
    );

    console.log("Authority:", keypair.publicKey.toString());
    console.log("PlatformConfig PDA:", platformConfig.toString());
    console.log("Sale Inventory Token Account:", saleInventoryTokenAccount.toString());
    console.log("USDT Treasury Token Account:", usdtTreasuryTokenAccount.toString());
    console.log("TANI Treasury Token Account:", taniTreasuryTokenAccount.toString());

    // Rate: 10 TANI per 1 USDT
    const TANI_PER_USDT = new anchor.BN(10);

    console.log("\nInitializing Z4 Platform...");

    const tx = await program.methods
        .initialize(TANI_PER_USDT)
        .accounts({
            platformConfig,
            authority: keypair.publicKey,
            taniMint: TANI_MINT,
            usdtMint: USDT_MINT,
            saleInventory: saleInventoryTokenAccount,
            usdtTreasury: usdtTreasuryTokenAccount,
            taniTreasury: taniTreasuryTokenAccount,
            systemProgram: anchor.web3.SystemProgram.programId,
        })
        .signers([keypair])
        .rpc();

    console.log("Platform initialized!");
    console.log("Transaction:", tx);
    console.log("Explorer:", `https://explorer.solana.com/tx/${tx}?cluster=devnet`);

    console.log("\n========================================");
    console.log("SIMPAN KE .env BACKEND:");
    console.log("========================================");
    console.log(`PLATFORM_CONFIG_PDA=${platformConfig.toString()}`);
    console.log("========================================");
}

main().catch(console.error);
