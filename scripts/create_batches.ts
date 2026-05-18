import * as anchor from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
import {
    createAssociatedTokenAccountInstruction,
    getAccount,
    getAssociatedTokenAddress,
} from "@solana/spl-token";
import crypto from "crypto";
import fs from "fs";

function uuidToBytes(uuid: string): number[] {
    const hex = uuid.replace(/-/g, "").toLowerCase();
    if (hex.length !== 32) throw new Error(`Invalid UUID: ${uuid}`);
    const out: number[] = [];
    for (let i = 0; i < 32; i += 2) out.push(parseInt(hex.slice(i, i + 2), 16));
    return out;
}

async function ensureAtaExists(params: {
    provider: anchor.AnchorProvider;
    mint: PublicKey;
    owner: PublicKey;
}): Promise<PublicKey> {
    const connection = params.provider.connection;
    const payer = params.provider.wallet.publicKey;
    const ata = await getAssociatedTokenAddress(params.mint, params.owner, true);
    try {
        await getAccount(connection, ata);
        return ata;
    } catch {
        const ix = createAssociatedTokenAccountInstruction(
            payer,
            ata,
            params.owner,
            params.mint
        );
        const tx = new anchor.web3.Transaction().add(ix);
        await params.provider.sendAndConfirm(tx, []);
        return ata;
    }
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

    // NOTE: ideally read this from PlatformConfig, but keeping explicit here.
    const TANI_MINT = new PublicKey("82uRtk77equ3QPbRdkzU7Hu5XKWt5ryAB9nGP8djRwSD");

    const batchUuid = crypto.randomUUID();
    const plotId = "C7";
    const targetTaniAtomic = new anchor.BN(100_000_000_000); // example
    const minFundedPct = 70;
    const endTime = new anchor.BN(Math.floor(Date.now() / 1000) + 7 * 24 * 60 * 60);

    const batchUuidBytes = uuidToBytes(batchUuid);
    const [batchStatePda] = PublicKey.findProgramAddressSync(
        [Buffer.from("batch_state"), Buffer.from(batchUuidBytes)],
        program.programId
    );
    const [batchVaultPda] = PublicKey.findProgramAddressSync(
        [Buffer.from("batch_vault"), Buffer.from(batchUuidBytes)],
        program.programId
    );
    const [platformConfig] = PublicKey.findProgramAddressSync(
        [Buffer.from("platform_config")],
        program.programId
    );

    console.log("Admin wallet:", keypair.publicKey.toString());
    console.log("Batch UUID:", batchUuid);
    console.log("BatchState PDA:", batchStatePda.toString());
    console.log("BatchVault PDA:", batchVaultPda.toString());

    console.log("\nCreating batch...");
    const tx = await program.methods
        .createBatch(batchUuidBytes, plotId, targetTaniAtomic, minFundedPct, endTime)
        .accounts({
            batchState: batchStatePda,
            batchVault: batchVaultPda,
            platformConfig,
            authority: keypair.publicKey,
            systemProgram: anchor.web3.SystemProgram.programId,
        })
        .signers([keypair])
        .rpc();

    console.log("Tx:", tx);
    console.log(`Explorer: https://explorer.solana.com/tx/${tx}?cluster=devnet`);

    console.log("\nEnsuring vault ATA exists...");
    const vaultAta = await ensureAtaExists({ provider, mint: TANI_MINT, owner: batchVaultPda });
    console.log("Vault ATA:", vaultAta.toString());

    console.log("\n========================================");
    console.log("BATCH CREATED");
    console.log("========================================");
    console.log(`BATCH_UUID=${batchUuid}`);
    console.log(`BATCH_STATE_PDA=${batchStatePda.toString()}`);
    console.log(`BATCH_VAULT_PDA=${batchVaultPda.toString()}`);
    console.log(`BATCH_VAULT_ATA=${vaultAta.toString()}`);
}

main().catch(console.error);