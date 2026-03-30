import * as anchor from "@coral-xyz/anchor";
import { PublicKey, SystemProgram } from "@solana/web3.js";
import fs from "fs";

async function main() {
    const connection = new anchor.web3.Connection(
        "https://api.devnet.solana.com",
        "confirmed"
    );

    const keypairFile = fs.readFileSync("/root/.config/solana/id.json", "utf-8");
    const keypairData = JSON.parse(keypairFile);
    const keypair = anchor.web3.Keypair.fromSecretKey(
        Uint8Array.from(keypairData)
    );

    const wallet = new anchor.Wallet(keypair);
    const provider = new anchor.AnchorProvider(connection, wallet, {
        commitment: "confirmed",
    });
    anchor.setProvider(provider);

    const idl = JSON.parse(
        fs.readFileSync("./target/idl/z4_contracts.json", "utf-8")
    );

    const programId = new PublicKey("8tUz3PDatBckE2FPAmFx4UUDV59SustzdmcwS7sLpbi1");
    const program = new anchor.Program(idl, provider);

    console.log("Admin wallet:", keypair.publicKey.toString());
    console.log("Program ID:", programId.toString());

    const batches = [
        { id: 1, totalUnits: 40000, pricePerUnit: 0.01 },
        { id: 2, totalUnits: 40000, pricePerUnit: 0.01 },
        { id: 3, totalUnits: 40000, pricePerUnit: 0.01 },
    ];

    for (const batch of batches) {
        const batchId = new anchor.BN(batch.id);
        const totalUnits = new anchor.BN(batch.totalUnits);
        const pricePerUnit = new anchor.BN(
            batch.pricePerUnit * anchor.web3.LAMPORTS_PER_SOL
        );

        const [batchPDA] = PublicKey.findProgramAddressSync(
            [Buffer.from("batch"), batchId.toArrayLike(Buffer, "le", 8)],
            programId
        );

        const [vaultPDA] = PublicKey.findProgramAddressSync(
            [Buffer.from("vault"), batchId.toArrayLike(Buffer, "le", 8)],
            programId
        );

        console.log(`\nMembuat Batch ${batch.id}...`);
        console.log("Batch PDA:", batchPDA.toString());
        console.log("Vault PDA:", vaultPDA.toString());

        try {
            const tx = await program.methods
                .createBatch(batchId, totalUnits, pricePerUnit)
                .accounts({
                    batch: batchPDA,
                    vault: vaultPDA,
                    authority: keypair.publicKey,
                    systemProgram: SystemProgram.programId,
                })
                .signers([keypair])
                .rpc();

            console.log(`Batch ${batch.id} berhasil!`);
            console.log("Tx:", tx);
            console.log(`Explorer: https://explorer.solana.com/tx/${tx}?cluster=devnet`);
        } catch (err) {
            console.error(`Error batch ${batch.id}:`, err);
        }
    }

    console.log("\n========================================");
    console.log("Semua batch selesai!");
    console.log("========================================");
}

main().catch(console.error);