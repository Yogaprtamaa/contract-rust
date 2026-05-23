module.exports = {
  apps: [
    {
      name: "z4-rate-updater",
      cwd: "/root/clonez4/z4-contracts",   // so .env + ./target/idl paths resolve
      script: "npx",
      args: "ts-node scripts/auto_update_rate.ts",
      cron_restart: "0 */6 * * *",   // every 6 hours: 00:00, 06:00, 12:00, 18:00
      autorestart: false,            // don't restart on exit — it's a one-shot script
      watch: false,
      env: {
        NODE_ENV: "production",
      },
    },
  ],
};
