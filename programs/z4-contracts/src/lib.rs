mod events;
use events::*;

use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{self, Burn, Mint, MintTo, Token, TokenAccount, Transfer};

declare_id!("8tUz3PDatBckE2FPAmFx4UUDV59SustzdmcwS7sLpbi1");

const PLATFORM_CONFIG_SEED: &[u8] = b"platform_config";
const SALE_AUTHORITY_SEED: &[u8] = b"sale_authority";
const BATCH_STATE_SEED: &[u8] = b"batch_state";
const BATCH_VAULT_SEED: &[u8] = b"batch_vault";
const ALLOCATION_STATE_SEED: &[u8] = b"allocation";
const NFT_MINT_SEED: &[u8] = b"nft_mint";

fn uuid_to_string(uuid: &[u8; 16]) -> String {
    // RFC4122 canonical format: 8-4-4-4-12 (hex)
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        uuid[0],
        uuid[1],
        uuid[2],
        uuid[3],
        uuid[4],
        uuid[5],
        uuid[6],
        uuid[7],
        uuid[8],
        uuid[9],
        uuid[10],
        uuid[11],
        uuid[12],
        uuid[13],
        uuid[14],
        uuid[15]
    )
}

fn str_to_fixed_bytes<const N: usize>(value: &str) -> [u8; N] {
    let mut out = [0u8; N];
    let bytes = value.as_bytes();
    let len = bytes.len().min(N);
    out[..len].copy_from_slice(&bytes[..len]);
    out
}

// ─── State ─────────────────────────────────────────────────────────────────

#[account]
pub struct PlatformConfig {
    pub authority: Pubkey,
    pub tani_mint: Pubkey,
    pub usdt_mint: Pubkey,
    pub sale_inventory: Pubkey,
    pub usdt_treasury: Pubkey,
    pub tani_treasury: Pubkey,
    pub tani_per_usdt: u64,
    pub token_sale_active: bool,
    pub allocation_active: bool,
    pub total_tani_sold: u64,
    pub total_tani_burned: u64,
    pub bump: u8,
}

impl PlatformConfig {
    pub const LEN: usize = 8 + 32 + 32 + 32 + 32 + 32 + 32 + 8 + 1 + 1 + 8 + 8 + 1;
}

#[account]
pub struct BatchState {
    pub batch_uuid: [u8; 16],
    pub plot_id: [u8; 32],
    pub authority: Pubkey,
    pub target_tani_atomic: u64,
    pub min_funded_pct: u8,
    pub funded_tani_atomic: u64,
    pub status: u8,
    pub start_time: i64,
    pub end_time: i64,
    pub finalized_at: i64,
    pub bump: u8,
    pub vault_bump: u8,
}

impl BatchState {
    pub const LEN: usize = 8
        + 16
        + 32
        + 32
        + 8
        + 1
        + 8
        + 1
        + 8
        + 8
        + 8
        + 1
        + 1;

    pub const STATUS_OPEN: u8 = 0;
    pub const STATUS_SUCCESS: u8 = 1;
    pub const STATUS_FAILED: u8 = 2;
}

#[account]
pub struct AllocationState {
    pub batch_uuid: [u8; 16],
    pub allocation_uuid: [u8; 16],
    pub wallet: Pubkey,
    pub plot_id: [u8; 32],
    pub tani_amount_atomic: u64,
    pub minted: bool,
    pub refunded: bool,
    pub nft_mint: Pubkey,
    pub created_at: i64,
    pub bump: u8,
}

impl AllocationState {
    pub const LEN: usize = 8
        + 16
        + 16
        + 32
        + 32
        + 8
        + 1
        + 1
        + 32
        + 8
        + 1;
}

// ─── Program ───────────────────────────────────────────────────────────────

#[program]
pub mod z4_contracts {
    use super::*;

    // ─── Platform Admin ──────────────────────────────────────

    pub fn initialize(ctx: Context<Initialize>, tani_per_usdt: u64) -> Result<()> {
        let config = &mut ctx.accounts.platform_config;
        config.authority = ctx.accounts.authority.key();
        config.tani_mint = ctx.accounts.tani_mint.key();
        config.usdt_mint = ctx.accounts.usdt_mint.key();
        config.sale_inventory = ctx.accounts.sale_inventory.key();
        config.usdt_treasury = ctx.accounts.usdt_treasury.key();
        config.tani_treasury = ctx.accounts.tani_treasury.key();
        config.tani_per_usdt = tani_per_usdt;
        config.token_sale_active = true;
        config.allocation_active = true;
        config.total_tani_sold = 0;
        config.total_tani_burned = 0;
        config.bump = ctx.bumps.platform_config;
        msg!("Z4 Platform initialized");
        Ok(())
    }

    pub fn update_rate(ctx: Context<AdminAction>, new_rate: u64) -> Result<()> {
        require!(new_rate > 0, Z4Error::InvalidRate);
        ctx.accounts.platform_config.tani_per_usdt = new_rate;
        msg!("Rate updated: {} TANI per USDT", new_rate);
        Ok(())
    }

    pub fn toggle_token_sale(ctx: Context<AdminAction>, active: bool) -> Result<()> {
        ctx.accounts.platform_config.token_sale_active = active;
        msg!("Token Sale: {}", if active { "AKTIF" } else { "NONAKTIF" });
        Ok(())
    }

    pub fn toggle_allocation(ctx: Context<AdminAction>, active: bool) -> Result<()> {
        ctx.accounts.platform_config.allocation_active = active;
        msg!("Allocation: {}", if active { "AKTIF" } else { "NONAKTIF" });
        Ok(())
    }

    // ─── Flow 1: Token Sale (USDT → $TANI) ──────────────────

    pub fn buy_tani(ctx: Context<BuyTani>, usdt_amount: u64) -> Result<()> {
        let config = &mut ctx.accounts.platform_config;

        require!(config.token_sale_active, Z4Error::TokenSaleInactive);
        require!(usdt_amount > 0, Z4Error::InvalidUsdtAmount);

        let tani_amount = usdt_amount
            .checked_mul(config.tani_per_usdt)
            .ok_or(Z4Error::Overflow)?
            .checked_mul(1000)
            .ok_or(Z4Error::Overflow)?;

        require!(
            ctx.accounts.sale_inventory.amount >= tani_amount,
            Z4Error::InsufficientInventory
        );

        // Transfer USDT → Treasury
        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.buyer_usdt_account.to_account_info(),
                    to: ctx.accounts.usdt_treasury.to_account_info(),
                    authority: ctx.accounts.buyer.to_account_info(),
                },
            ),
            usdt_amount,
        )?;

        // Transfer $TANI → Buyer (via sale_authority PDA)
        let bump = ctx.bumps.sale_authority;
        let seeds = &[SALE_AUTHORITY_SEED, &[bump]];
        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.sale_inventory.to_account_info(),
                    to: ctx.accounts.buyer_tani_account.to_account_info(),
                    authority: ctx.accounts.sale_authority.to_account_info(),
                },
                &[seeds],
            ),
            tani_amount,
        )?;

        config.total_tani_sold = config
            .total_tani_sold
            .checked_add(tani_amount)
            .ok_or(Z4Error::Overflow)?;

        emit!(TokenSaleEvent {
            wallet: ctx.accounts.buyer.key(),
            usdt_amount,
            tani_received: tani_amount,
            rate: config.tani_per_usdt,
            timestamp: Clock::get()?.unix_timestamp,
        });

        msg!(
            "Token Sale: {} USDT → {} TANI wallet={}",
            usdt_amount,
            tani_amount,
            ctx.accounts.buyer.key()
        );
        Ok(())
    }

    // ─── Flow 2: Batch Funding ───────────────────────────────

    pub fn create_batch(
        ctx: Context<CreateBatch>,
        batch_uuid: [u8; 16],
        plot_id: String,
        target_tani_atomic: u64,
        min_funded_pct: u8,
        end_time: i64,
    ) -> Result<()> {
        require!(target_tani_atomic > 0, Z4Error::InvalidTargetAmount);
        require!(min_funded_pct > 0 && min_funded_pct <= 100, Z4Error::InvalidFundedPct);

        let now = Clock::get()?.unix_timestamp;
        require!(end_time > now, Z4Error::InvalidEndTime);

        let batch = &mut ctx.accounts.batch_state;
        batch.batch_uuid = batch_uuid;
        batch.plot_id = str_to_fixed_bytes::<32>(&plot_id);
        batch.authority = ctx.accounts.authority.key();
        batch.target_tani_atomic = target_tani_atomic;
        batch.min_funded_pct = min_funded_pct;
        batch.funded_tani_atomic = 0;
        batch.status = BatchState::STATUS_OPEN;
        batch.start_time = now;
        batch.end_time = end_time;
        batch.finalized_at = 0;
        batch.bump = ctx.bumps.batch_state;
        batch.vault_bump = ctx.bumps.batch_vault;

        let batch_str = uuid_to_string(&batch_uuid);
        msg!("BatchCreated: batch={} plot={} target={} min_pct={} end_time={}", batch_str, plot_id, target_tani_atomic, min_funded_pct, end_time);
        Ok(())
    }

    pub fn allocate_to_batch(
        ctx: Context<AllocateToBatch>,
        batch_uuid: [u8; 16],
        allocation_uuid: [u8; 16],
        plot_id: String,
        tani_amount_atomic: u64,
    ) -> Result<()> {
        require!(tani_amount_atomic > 0, Z4Error::InvalidAllocationAmount);

        let config = &ctx.accounts.platform_config;
        require!(config.allocation_active, Z4Error::AllocationInactive);

        let now = Clock::get()?.unix_timestamp;
        let batch = &mut ctx.accounts.batch_state;
        require!(batch.status == BatchState::STATUS_OPEN, Z4Error::BatchNotOpen);
        require!(now < batch.end_time, Z4Error::BatchExpired);

        let allocation = &mut ctx.accounts.allocation_state;
        if allocation.tani_amount_atomic != 0 {
            // Idempotent retry: allocation already exists, so don't transfer again.
            require!(allocation.wallet == ctx.accounts.user.key(), Z4Error::Unauthorized);
            require!(allocation.allocation_uuid == allocation_uuid, Z4Error::AllocationMismatch);
            require!(allocation.batch_uuid == batch_uuid, Z4Error::AllocationMismatch);
            require!(allocation.tani_amount_atomic == tani_amount_atomic, Z4Error::AllocationMismatch);
            return Ok(());
        }

        require!(
            ctx.accounts.user_tani_account.amount >= tani_amount_atomic,
            Z4Error::InsufficientTani
        );

        // Transfer $TANI user → batch vault token account
        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.user_tani_account.to_account_info(),
                    to: ctx.accounts.vault_tani_account.to_account_info(),
                    authority: ctx.accounts.user.to_account_info(),
                },
            ),
            tani_amount_atomic,
        )?;

        batch.funded_tani_atomic = batch
            .funded_tani_atomic
            .checked_add(tani_amount_atomic)
            .ok_or(Z4Error::Overflow)?;

        allocation.batch_uuid = batch_uuid;
        allocation.allocation_uuid = allocation_uuid;
        allocation.wallet = ctx.accounts.user.key();
        allocation.plot_id = str_to_fixed_bytes::<32>(&plot_id);
        allocation.tani_amount_atomic = tani_amount_atomic;
        allocation.minted = false;
        allocation.refunded = false;
        allocation.nft_mint = Pubkey::default();
        allocation.created_at = now;
        allocation.bump = ctx.bumps.allocation_state;

        let batch_str = uuid_to_string(&batch_uuid);
        let alloc_str = uuid_to_string(&allocation_uuid);

        emit!(AllocationEvent {
            wallet: ctx.accounts.user.key(),
            batch_id: batch_str.clone(),
            allocation_id: alloc_str.clone(),
            tani_spent: tani_amount_atomic,
            timestamp: now,
        });

        msg!(
            "Allocation: batch={} allocation={} plot={} tani={} wallet={}",
            batch_str,
            alloc_str,
            plot_id,
            tani_amount_atomic,
            ctx.accounts.user.key()
        );
        Ok(())
    }

    pub fn finalize_batch(ctx: Context<FinalizeBatch>, batch_uuid: [u8; 16]) -> Result<()> {
        let config = &mut ctx.accounts.platform_config;
        require!(config.authority == ctx.accounts.authority.key(), Z4Error::Unauthorized);

        let now = Clock::get()?.unix_timestamp;
        let batch = &mut ctx.accounts.batch_state;
        require!(batch.status == BatchState::STATUS_OPEN, Z4Error::BatchAlreadyFinalized);

        let target = batch.target_tani_atomic;
        require!(target > 0, Z4Error::InvalidTargetAmount);

        // Use vault amount as on-chain truth.
        let funded = ctx.accounts.vault_tani_account.amount;
        batch.funded_tani_atomic = funded;
        let funded_pct: u8 = ((funded
            .checked_mul(100)
            .ok_or(Z4Error::Overflow)?
            .checked_div(target)
            .ok_or(Z4Error::Overflow)?)
            .min(100)) as u8;

        let is_success = funded_pct >= batch.min_funded_pct;
        let is_expired = now >= batch.end_time;

        if is_success {
            batch.status = BatchState::STATUS_SUCCESS;
        } else if is_expired {
            batch.status = BatchState::STATUS_FAILED;
        } else {
            return Err(Z4Error::BatchNotEnded.into());
        }
        batch.finalized_at = now;

        let batch_str = uuid_to_string(&batch_uuid);

        if batch.status == BatchState::STATUS_SUCCESS {
            let treasury_amount = funded
                .checked_mul(70)
                .ok_or(Z4Error::Overflow)?
                .checked_div(100)
                .ok_or(Z4Error::Overflow)?;
            let burn_amount = funded
                .checked_sub(treasury_amount)
                .ok_or(Z4Error::Overflow)?;

            let vault_bump = batch.vault_bump;
            let seeds = &[BATCH_VAULT_SEED, batch_uuid.as_ref(), &[vault_bump]];

            // Transfer 70% to treasury
            token::transfer(
                CpiContext::new_with_signer(
                    ctx.accounts.token_program.to_account_info(),
                    Transfer {
                        from: ctx.accounts.vault_tani_account.to_account_info(),
                        to: ctx.accounts.tani_treasury.to_account_info(),
                        authority: ctx.accounts.batch_vault.to_account_info(),
                    },
                    &[seeds],
                ),
                treasury_amount,
            )?;

            // Burn 30% from vault
            token::burn(
                CpiContext::new_with_signer(
                    ctx.accounts.token_program.to_account_info(),
                    Burn {
                        mint: ctx.accounts.tani_mint.to_account_info(),
                        from: ctx.accounts.vault_tani_account.to_account_info(),
                        authority: ctx.accounts.batch_vault.to_account_info(),
                    },
                    &[seeds],
                ),
                burn_amount,
            )?;

            config.total_tani_burned = config
                .total_tani_burned
                .checked_add(burn_amount)
                .ok_or(Z4Error::Overflow)?;

            msg!("BatchFinalized: batch={} result=success pct={}", batch_str, funded_pct);
        } else {
            msg!("BatchFinalized: batch={} result=failed pct={}", batch_str, funded_pct);
        }

        emit!(BatchFinalizedEvent {
            batch_id: batch_str,
            result: if batch.status == BatchState::STATUS_SUCCESS {
                "success".to_string()
            } else {
                "failed".to_string()
            },
            funded_pct,
            funded_tani: funded,
            timestamp: now,
        });

        Ok(())
    }

    pub fn mint_nft_for_allocation(
        ctx: Context<MintNftForAllocation>,
        batch_uuid: [u8; 16],
        allocation_uuid: [u8; 16],
        plot_id: String,
        _metadata_uri: String,
    ) -> Result<()> {
        let config = &ctx.accounts.platform_config;
        require!(config.authority == ctx.accounts.authority.key(), Z4Error::Unauthorized);

        let batch = &ctx.accounts.batch_state;
        require!(batch.status == BatchState::STATUS_SUCCESS, Z4Error::BatchNotSuccessful);

        let allocation = &mut ctx.accounts.allocation_state;
        require!(allocation.batch_uuid == batch_uuid, Z4Error::AllocationMismatch);
        require!(allocation.allocation_uuid == allocation_uuid, Z4Error::AllocationMismatch);
        require!(allocation.wallet == ctx.accounts.user_wallet.key(), Z4Error::AllocationMismatch);
        require!(!allocation.refunded, Z4Error::AlreadyRefunded);
        require!(!allocation.minted, Z4Error::AlreadyMinted);

        // Mint a 1-of-1 SPL token (decimals=0) as the NFT placeholder.
        let batch_bump = batch.bump;
        let batch_seeds = &[BATCH_STATE_SEED, batch_uuid.as_ref(), &[batch_bump]];

        token::mint_to(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                MintTo {
                    mint: ctx.accounts.nft_mint.to_account_info(),
                    to: ctx.accounts.user_nft_ata.to_account_info(),
                    authority: ctx.accounts.batch_state.to_account_info(),
                },
                &[batch_seeds],
            ),
            1,
        )?;

        allocation.minted = true;
        allocation.nft_mint = ctx.accounts.nft_mint.key();

        let batch_str = uuid_to_string(&batch_uuid);
        let alloc_str = uuid_to_string(&allocation_uuid);
        msg!(
            "NFTMinted: batch={} allocation={} plot={} nft={} wallet={}",
            batch_str,
            alloc_str,
            plot_id,
            ctx.accounts.nft_mint.key(),
            ctx.accounts.user_wallet.key()
        );
        Ok(())
    }

    pub fn claim_refund(
        ctx: Context<ClaimRefund>,
        batch_uuid: [u8; 16],
        allocation_uuid: [u8; 16],
    ) -> Result<()> {
        let batch = &ctx.accounts.batch_state;
        require!(batch.status == BatchState::STATUS_FAILED, Z4Error::BatchNotFailed);

        let allocation = &mut ctx.accounts.allocation_state;
        require!(allocation.batch_uuid == batch_uuid, Z4Error::AllocationMismatch);
        require!(allocation.allocation_uuid == allocation_uuid, Z4Error::AllocationMismatch);
        require!(allocation.wallet == ctx.accounts.user.key(), Z4Error::Unauthorized);
        require!(!allocation.minted, Z4Error::AlreadyMinted);
        require!(!allocation.refunded, Z4Error::AlreadyRefunded);

        let refund_amount = allocation.tani_amount_atomic;
        require!(refund_amount > 0, Z4Error::InvalidAllocationAmount);

        let vault_bump = ctx.accounts.batch_state.vault_bump;
        let seeds = &[BATCH_VAULT_SEED, batch_uuid.as_ref(), &[vault_bump]];

        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.vault_tani_account.to_account_info(),
                    to: ctx.accounts.user_tani_account.to_account_info(),
                    authority: ctx.accounts.batch_vault.to_account_info(),
                },
                &[seeds],
            ),
            refund_amount,
        )?;

        allocation.refunded = true;

        let now = Clock::get()?.unix_timestamp;
        let batch_str = uuid_to_string(&batch_uuid);
        let alloc_str = uuid_to_string(&allocation_uuid);

        emit!(RefundClaimedEvent {
            wallet: ctx.accounts.user.key(),
            batch_id: batch_str.clone(),
            allocation_id: alloc_str.clone(),
            tani_refunded: refund_amount,
            timestamp: now,
        });

        msg!(
            "Refunded: batch={} allocation={} wallet={} tani={}",
            batch_str,
            alloc_str,
            ctx.accounts.user.key(),
            refund_amount
        );
        Ok(())
    }
}

// ─── Contexts ──────────────────────────────────────────────────────────────

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        payer = authority,
        space = PlatformConfig::LEN,
        seeds = [PLATFORM_CONFIG_SEED],
        bump,
    )]
    pub platform_config: Account<'info, PlatformConfig>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub tani_mint: Account<'info, Mint>,
    pub usdt_mint: Account<'info, Mint>,
    #[account(mut)]
    pub sale_inventory: Account<'info, TokenAccount>,
    #[account(mut)]
    pub usdt_treasury: Account<'info, TokenAccount>,
    #[account(mut)]
    pub tani_treasury: Account<'info, TokenAccount>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct AdminAction<'info> {
    #[account(
        mut,
        seeds = [PLATFORM_CONFIG_SEED],
        bump = platform_config.bump,
        has_one = authority,
    )]
    pub platform_config: Account<'info, PlatformConfig>,
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct BuyTani<'info> {
    #[account(mut, seeds = [PLATFORM_CONFIG_SEED], bump = platform_config.bump)]
    pub platform_config: Account<'info, PlatformConfig>,
    #[account(mut)]
    pub buyer: Signer<'info>,
    #[account(
        mut,
        constraint = buyer_usdt_account.owner == buyer.key(),
        constraint = buyer_usdt_account.mint == platform_config.usdt_mint,
    )]
    pub buyer_usdt_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = buyer_tani_account.owner == buyer.key(),
        constraint = buyer_tani_account.mint == platform_config.tani_mint,
    )]
    pub buyer_tani_account: Account<'info, TokenAccount>,
    #[account(mut, constraint = usdt_treasury.key() == platform_config.usdt_treasury)]
    pub usdt_treasury: Account<'info, TokenAccount>,
    #[account(mut, constraint = sale_inventory.key() == platform_config.sale_inventory)]
    pub sale_inventory: Account<'info, TokenAccount>,
    #[account(seeds = [SALE_AUTHORITY_SEED], bump)]
    /// CHECK: PDA authority
    pub sale_authority: AccountInfo<'info>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
#[instruction(batch_uuid: [u8; 16])]
pub struct CreateBatch<'info> {
    #[account(
        init,
        payer = authority,
        space = BatchState::LEN,
        seeds = [BATCH_STATE_SEED, batch_uuid.as_ref()],
        bump,
    )]
    pub batch_state: Account<'info, BatchState>,
    #[account(seeds = [BATCH_VAULT_SEED, batch_uuid.as_ref()], bump)]
    /// CHECK: PDA authority for vault token account
    pub batch_vault: AccountInfo<'info>,
    #[account(seeds = [PLATFORM_CONFIG_SEED], bump = platform_config.bump, has_one = authority)]
    pub platform_config: Account<'info, PlatformConfig>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(batch_uuid: [u8; 16], allocation_uuid: [u8; 16])]
pub struct AllocateToBatch<'info> {
    #[account(mut, seeds = [BATCH_STATE_SEED, batch_uuid.as_ref()], bump = batch_state.bump)]
    pub batch_state: Account<'info, BatchState>,

    #[account(
        init_if_needed,
        payer = user,
        space = AllocationState::LEN,
        seeds = [ALLOCATION_STATE_SEED, batch_uuid.as_ref(), allocation_uuid.as_ref()],
        bump,
    )]
    pub allocation_state: Account<'info, AllocationState>,

    #[account(seeds = [BATCH_VAULT_SEED, batch_uuid.as_ref()], bump = batch_state.vault_bump)]
    /// CHECK: PDA authority for vault token account
    pub batch_vault: AccountInfo<'info>,

    #[account(seeds = [PLATFORM_CONFIG_SEED], bump = platform_config.bump)]
    pub platform_config: Account<'info, PlatformConfig>,

    #[account(
        mut,
        constraint = user_tani_account.owner == user.key(),
        constraint = user_tani_account.mint == platform_config.tani_mint,
    )]
    pub user_tani_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = vault_tani_account.owner == batch_vault.key(),
        constraint = vault_tani_account.mint == platform_config.tani_mint,
    )]
    pub vault_tani_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(batch_uuid: [u8; 16])]
pub struct FinalizeBatch<'info> {
    #[account(mut, seeds = [BATCH_STATE_SEED, batch_uuid.as_ref()], bump = batch_state.bump)]
    pub batch_state: Account<'info, BatchState>,
    #[account(seeds = [BATCH_VAULT_SEED, batch_uuid.as_ref()], bump = batch_state.vault_bump)]
    /// CHECK: PDA authority for vault token account
    pub batch_vault: AccountInfo<'info>,
    #[account(
        mut,
        constraint = vault_tani_account.owner == batch_vault.key(),
        constraint = vault_tani_account.mint == platform_config.tani_mint,
    )]
    pub vault_tani_account: Account<'info, TokenAccount>,
    #[account(mut, constraint = tani_treasury.key() == platform_config.tani_treasury)]
    pub tani_treasury: Account<'info, TokenAccount>,
    #[account(constraint = tani_mint.key() == platform_config.tani_mint)]
    pub tani_mint: Account<'info, Mint>,
    #[account(mut, seeds = [PLATFORM_CONFIG_SEED], bump = platform_config.bump)]
    pub platform_config: Account<'info, PlatformConfig>,
    pub authority: Signer<'info>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
#[instruction(batch_uuid: [u8; 16], allocation_uuid: [u8; 16])]
pub struct MintNftForAllocation<'info> {
    #[account(seeds = [PLATFORM_CONFIG_SEED], bump = platform_config.bump)]
    pub platform_config: Account<'info, PlatformConfig>,
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        mut,
        seeds = [BATCH_STATE_SEED, batch_uuid.as_ref()],
        bump = batch_state.bump,
    )]
    pub batch_state: Account<'info, BatchState>,

    #[account(
        mut,
        seeds = [ALLOCATION_STATE_SEED, batch_uuid.as_ref(), allocation_uuid.as_ref()],
        bump = allocation_state.bump,
    )]
    pub allocation_state: Account<'info, AllocationState>,

    /// CHECK: user wallet that receives the NFT
    pub user_wallet: UncheckedAccount<'info>,

    #[account(
        init,
        payer = authority,
        seeds = [NFT_MINT_SEED, batch_uuid.as_ref(), allocation_uuid.as_ref()],
        bump,
        mint::decimals = 0,
        mint::authority = batch_state,
    )]
    pub nft_mint: Account<'info, Mint>,

    #[account(
        init_if_needed,
        payer = authority,
        associated_token::mint = nft_mint,
        associated_token::authority = user_wallet,
    )]
    pub user_nft_ata: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(batch_uuid: [u8; 16], allocation_uuid: [u8; 16])]
pub struct ClaimRefund<'info> {
    #[account(seeds = [BATCH_STATE_SEED, batch_uuid.as_ref()], bump = batch_state.bump)]
    pub batch_state: Account<'info, BatchState>,
    #[account(
        mut,
        seeds = [ALLOCATION_STATE_SEED, batch_uuid.as_ref(), allocation_uuid.as_ref()],
        bump = allocation_state.bump,
    )]
    pub allocation_state: Account<'info, AllocationState>,
    #[account(seeds = [BATCH_VAULT_SEED, batch_uuid.as_ref()], bump = batch_state.vault_bump)]
    /// CHECK: PDA authority for vault token account
    pub batch_vault: AccountInfo<'info>,
    #[account(
        mut,
        constraint = vault_tani_account.owner == batch_vault.key(),
        constraint = vault_tani_account.mint == platform_config.tani_mint,
    )]
    pub vault_tani_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = user_tani_account.owner == user.key(),
        constraint = user_tani_account.mint == platform_config.tani_mint,
    )]
    pub user_tani_account: Account<'info, TokenAccount>,
    #[account(seeds = [PLATFORM_CONFIG_SEED], bump = platform_config.bump)]
    pub platform_config: Account<'info, PlatformConfig>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub token_program: Program<'info, Token>,
}

// ─── Errors ────────────────────────────────────────────────────────────────

#[error_code]
pub enum Z4Error {
    // Token Sale
    #[msg("Sale inventory tidak cukup stok $TANI")]
    InsufficientInventory,
    #[msg("Jumlah USDT harus lebih dari 0")]
    InvalidUsdtAmount,
    #[msg("Rate tidak valid")]
    InvalidRate,
    #[msg("Token Sale tidak aktif")]
    TokenSaleInactive,

    // Batch
    #[msg("Batch tidak open")]
    BatchNotOpen,
    #[msg("Batch sudah berakhir")]
    BatchExpired,
    #[msg("Batch belum berakhir")]
    BatchNotEnded,
    #[msg("Batch sudah di-finalize")]
    BatchAlreadyFinalized,
    #[msg("Batch tidak sukses")]
    BatchNotSuccessful,
    #[msg("Batch belum gagal")]
    BatchNotFailed,
    #[msg("Target TANI tidak valid")]
    InvalidTargetAmount,
    #[msg("End time tidak valid")]
    InvalidEndTime,
    #[msg("min_funded_pct tidak valid")]
    InvalidFundedPct,

    // Allocation
    #[msg("Saldo $TANI tidak cukup")]
    InsufficientTani,
    #[msg("Allocation sedang tidak aktif")]
    AllocationInactive,
    #[msg("Jumlah allocation tidak valid")]
    InvalidAllocationAmount,
    #[msg("Allocation state mismatch")]
    AllocationMismatch,
    #[msg("Allocation sudah di-mint")]
    AlreadyMinted,
    #[msg("Allocation sudah di-refund")]
    AlreadyRefunded,

    // General
    #[msg("Overflow kalkulasi")]
    Overflow,
    #[msg("Tidak authorized")]
    Unauthorized,
}
