import * as anchor from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
import fs from "fs";

async function main() {
    const connection = new anchor.web3.Connection("https://api.devnet.solana.com", "confirmed");
    const keypair = anchor.web3.Keypair.fromSecretKey(
        Uint8Array.from(JSON.parse(fs.readFileSync("/root/.config/solana/id.json", "utf-8")))
    );
    const provider = new anchor.AnchorProvider(connection, new anchor.Wallet(keypair), { commitment: "confirmed" });
    anchor.setProvider(provider);

    const idl = JSON.parse(fs.readFileSync("./target/idl/z4_contracts.json", "utf-8"));
    const program = new anchor.Program(idl, provider);

    const [platformConfig] = PublicKey.findProgramAddressSync(
        [Buffer.from("platform_config")], program.programId
    );

    const NEW_SALE_INVENTORY = new PublicKey("EMZHThU4jweMEKPSpoUpDSe2CCDJDr3TMyV8NdVYDbcp");

    console.log("PlatformConfig PDA:", platformConfig.toString());
    console.log("New sale inventory:", NEW_SALE_INVENTORY.toString());

    const tx = await program.methods
        .setSaleInventory()
        .accounts({
            platformConfig,
            authority: keypair.publicKey,
            newSaleInventory: NEW_SALE_INVENTORY,
        })
        .rpc();

    console.log("Done! Tx:", tx);
    console.log("Explorer:", `https://explorer.solana.com/tx/${tx}?cluster=devnet`);
}

main().catch(console.error);
