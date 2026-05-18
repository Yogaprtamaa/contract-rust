import * as anchor from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
import { getAccount, getAssociatedTokenAddress } from "@solana/spl-token";
import crypto from "crypto";
import fs from "fs";

function uuidToBytes(uuid: string): number[] {
    const hex = uuid.replace(/-/g, "").toLowerCase();
    if (hex.length !== 32) throw new Error(`Invalid UUID: ${uuid}`);
    const out: number[] = [];
    for (let i = 0; i < 32; i += 2) out.push(parseInt(hex.slice(i, i + 2), 16));
    return out;
}

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

    const idl = JSON.parse(fs.readFileSync("./target/idl/z4_contracts.json", "utf-8"));
    const program = new anchor.Program(idl, provider);

    const TANI_MINT = new PublicKey("82uRtk77equ3QPbRdkzU7Hu5XKWt5ryAB9nGP8djRwSD");

    // Fill these with a real batch UUID created via scripts/create_batches.ts
    const batchUuid = process.env.BATCH_UUID;
    if (!batchUuid) throw new Error("Missing env BATCH_UUID");

    const allocationUuid = crypto.randomUUID();
    const plotId = "C7";
    const taniAmount = new anchor.BN(10_000_000_000); // example

    const batchUuidBytes = uuidToBytes(batchUuid);
    const allocationUuidBytes = uuidToBytes(allocationUuid);

    const [batchState] = PublicKey.findProgramAddressSync(
        [Buffer.from("batch_state"), Buffer.from(batchUuidBytes)],
        program.programId
    );
    const [batchVault] = PublicKey.findProgramAddressSync(
        [Buffer.from("batch_vault"), Buffer.from(batchUuidBytes)],
        program.programId
    );
    const [allocationState] = PublicKey.findProgramAddressSync(
        [Buffer.from("allocation"), Buffer.from(batchUuidBytes), Buffer.from(allocationUuidBytes)],
        program.programId
    );
    const [platformConfig] = PublicKey.findProgramAddressSync(
        [Buffer.from("platform_config")],
        program.programId
    );

    const userTaniAccount = await getAssociatedTokenAddress(TANI_MINT, keypair.publicKey);
    const vaultTaniAccount = await getAssociatedTokenAddress(TANI_MINT, batchVault, true);

    console.log("Batch UUID:", batchUuid);
    console.log("Allocation UUID:", allocationUuid);
    console.log("BatchState:", batchState.toString());
    console.log("AllocationState:", allocationState.toString());
    console.log("Vault ATA:", vaultTaniAccount.toString());

    console.log("\n=== SEBELUM ===");
    const userTaniBefore = await getAccount(connection, userTaniAccount);
    const vaultBefore = await getAccount(connection, vaultTaniAccount);
    console.log("User TANI:", userTaniBefore.amount.toString());
    console.log("Vault TANI:", vaultBefore.amount.toString());

    console.log(`\nAllocate ${taniAmount.toString()} TANI to batch...`);
    const tx = await program.methods
        .allocateToBatch(batchUuidBytes, allocationUuidBytes, plotId, taniAmount)
        .accounts({
            batchState,
            allocationState,
            batchVault,
            platformConfig,
            userTaniAccount,
            vaultTaniAccount,
            user: keypair.publicKey,
            tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
            systemProgram: anchor.web3.SystemProgram.programId,
        })
        .signers([keypair])
        .rpc();

    console.log("\nTransaction:", tx);
    console.log("Explorer: https://explorer.solana.com/tx/" + tx + "?cluster=devnet");

    console.log("\n=== SESUDAH ===");
    const userTaniAfter = await getAccount(connection, userTaniAccount);
    const vaultAfter = await getAccount(connection, vaultTaniAccount);
    console.log("User TANI:", userTaniAfter.amount.toString());
    console.log("Vault TANI:", vaultAfter.amount.toString());
}

main().catch(console.error);
