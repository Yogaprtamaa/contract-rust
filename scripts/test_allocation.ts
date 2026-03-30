import * as anchor from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
import { getAssociatedTokenAddress, getAccount } from "@solana/spl-token";
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
    const TANI_TREASURY_WALLET = new PublicKey("8d56NGYEsWiQ3EiqVCF2WAQmTDEyZS5io8q1Q4Ui4XgG");

    const [platformConfig] = PublicKey.findProgramAddressSync(
        [Buffer.from("platform_config")],
        program.programId
    );

    const userTaniAccount = await getAssociatedTokenAddress(TANI_MINT, keypair.publicKey);
    const taniTreasuryAccount = await getAssociatedTokenAddress(TANI_MINT, TANI_TREASURY_WALLET);

    console.log("=== SEBELUM ===");
    const userTaniBefore = await getAccount(connection, userTaniAccount);
    const treasuryBefore = await getAccount(connection, taniTreasuryAccount);
    console.log("User TANI:", userTaniBefore.amount.toString());
    console.log("Treasury TANI:", treasuryBefore.amount.toString());

    const taniAmount = new anchor.BN(100_000_000_000);
    const plotId = "C7";
    const nftId = "Z4-PLOT-C7-" + Date.now().toString();

    console.log("\nAlokasi plot " + plotId + " dengan 100 TANI...");
    console.log("NFT ID: " + nftId);
    console.log("Routing: 70 TANI ke Treasury, 30 TANI di-burn");

    const tx = await program.methods
        .allocatePlot(plotId, nftId, taniAmount)
        .accounts({
            platformConfig,
            user: keypair.publicKey,
            userTaniAccount,
            taniTreasury: taniTreasuryAccount,
            taniMint: TANI_MINT,
            tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
        })
        .signers([keypair])
        .rpc();

    console.log("\nTransaction:", tx);
    console.log("Explorer: https://explorer.solana.com/tx/" + tx + "?cluster=devnet");

    console.log("\n=== SESUDAH ===");
    const userTaniAfter = await getAccount(connection, userTaniAccount);
    const treasuryAfter = await getAccount(connection, taniTreasuryAccount);
    console.log("User TANI:", userTaniAfter.amount.toString());
    console.log("Treasury TANI:", treasuryAfter.amount.toString());

    const taniSpent = BigInt(userTaniBefore.amount) - BigInt(userTaniAfter.amount);
    const treasuryGain = BigInt(treasuryAfter.amount) - BigInt(treasuryBefore.amount);
    const burned = taniSpent - treasuryGain;

    console.log("\n=== ROUTING SUMMARY ===");
    console.log("TANI dipakai:", taniSpent.toString());
    console.log("Ke Treasury (70%):", treasuryGain.toString());
    console.log("Di-burn (30%):", burned.toString());
    console.log("\nAllocation Flow berhasil!");
}

main().catch(console.error);
