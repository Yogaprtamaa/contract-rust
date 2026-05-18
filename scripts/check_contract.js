const anchor = require("@coral-xyz/anchor");
const fs = require("fs");

async function main() {
    const connection = new anchor.web3.Connection(
        "https://api.devnet.solana.com", "confirmed"
    );
    const keypair = anchor.web3.Keypair.fromSecretKey(
        Uint8Array.from(JSON.parse(fs.readFileSync("/root/.config/solana/id.json", "utf-8")))
    );
    const provider = new anchor.AnchorProvider(connection, new anchor.Wallet(keypair), {});
    anchor.setProvider(provider);

    const idl = JSON.parse(fs.readFileSync("./target/idl/z4_contracts.json", "utf-8"));
    const program = new anchor.Program(idl, provider);

    const [pda] = anchor.web3.PublicKey.findProgramAddressSync(
        [Buffer.from("platform_config")],
        program.programId
    );

    const config = await program.account.platformConfig.fetch(pda);

    console.log("=== PLATFORM CONFIG ===");
    console.log("Authority     :", config.authority.toString());
    console.log("TANI Mint     :", config.taniMint.toString());
    console.log("USDT Mint     :", config.usdtMint.toString());
    console.log("Rate          :", config.taniPerUsdt.toString(), "TANI per USDT");
    console.log("Token Sale    :", config.tokenSaleActive ? "AKTIF" : "NONAKTIF");
    console.log("Allocation    :", config.allocationActive ? "AKTIF" : "NONAKTIF");
    console.log("Total Sold    :", config.totalTaniSold.toString());
    console.log("Total Burned  :", config.totalTaniBurned.toString());
}

main().catch(console.error);
