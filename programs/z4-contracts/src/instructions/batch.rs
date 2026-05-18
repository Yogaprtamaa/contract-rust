use anchor_lang::prelude::*;
use anchor_spl::token::{self, Burn, Token, TokenAccount, Transfer, Mint};
use crate::state::{BatchAccount, BatchStatus, AllocationReceipt, ReceiptStatus, PlatformConfig};
use crate::errors::Z4Error;
use crate::events::*;

// ─── Create Batch ──────────────────────────────────────────────────────────

#[derive(Accounts)]
#[instruction(batch_id: String)]
pub struct CreateBatch<'info> {
    #[account(
        init,
        payer = authority,
        space = BatchAccount::LEN,
        seeds = [b"batch", batch_id.as_bytes()],
        bump
    )]
    pub batch: Account<'info, BatchAccount>,

    /// CHECK: Vault PDA untuk tampung $TANI user
    #[account(
        mut,
        seeds = [b"batch_vault", batch_id.as_bytes()],
        bump
    )]
    pub batch_vault: AccountInfo<'info>,

    #[account(
        seeds = [b"platform_config"],
        bump = platform_config.bump,
        has_one = authority,
    )]
    pub platform_config: Account<'info, PlatformConfig>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn create_batch_handler(
    ctx: Context<CreateBatch>,
    batch_id: String,
    plot_id: String,
    tani_per_nft: u64,
    total_nft: u64,
    duration_days: i64,
) -> Result<()> {
    require!(total_nft > 0, Z4Error::InvalidNftQuantity);
    require!(tani_per_nft > 0, Z4Error::InvalidNftQuantity);

    let target_nft = total_nft
        .checked_mul(70)
        .ok_or(Z4Error::Overflow)?
        .checked_div(100)
        .ok_or(Z4Error::Overflow)?;

    let clock = Clock::get()?;
    let end_time = clock.unix_timestamp + (duration_days * 24 * 60 * 60);

    let batch = &mut ctx.accounts.batch;

    // Store batch_id as bytes
    let mut batch_id_bytes = [0u8; 20];
    let id_bytes = batch_id.as_bytes();
    let len = id_bytes.len().min(20);
    batch_id_bytes[..len].copy_from_slice(&id_bytes[..len]);
    batch.batch_id = batch_id_bytes;

    // Store plot_id as bytes
    let mut plot_id_bytes = [0u8; 10];
    let pid_bytes = plot_id.as_bytes();
    let plen = pid_bytes.len().min(10);
    plot_id_bytes[..plen].copy_from_slice(&pid_bytes[..plen]);
    batch.plot_id = plot_id_bytes;

    batch.authority = ctx.accounts.authority.key();
    batch.tani_per_nft = tani_per_nft;
    batch.total_nft = total_nft;
    batch.target_nft = target_nft;
    batch.filled_nft = 0;
    batch.total_tani_collected = 0;
    batch.status = BatchStatus::Open;
    batch.start_time = clock.unix_timestamp;
    batch.end_time = end_time;
    batch.bump = ctx.bumps.batch;
    batch.vault_bump = ctx.bumps.batch_vault;

    emit!(BatchCreated {
        batch_id: batch_id.clone(),
        plot_id: plot_id.clone(),
        total_nft,
        target_nft,
        tani_per_nft,
        end_time,
    });

    msg!("Batch {} dibuat: {} NFT @ {} TANI, target {}",
        batch_id, total_nft, tani_per_nft, target_nft);
    Ok(())
}

// ─── Join Batch ────────────────────────────────────────────────────────────

#[derive(Accounts)]
#[instruction(batch_id: String)]
pub struct JoinBatch<'info> {
    #[account(
        mut,
        seeds = [b"batch", batch_id.as_bytes()],
        bump = batch.bump,
    )]
    pub batch: Account<'info, BatchAccount>,

    #[account(
        init,
        payer = user,
        space = AllocationReceipt::LEN,
        seeds = [b"receipt", batch_id.as_bytes(), user.key().as_ref()],
        bump
    )]
    pub receipt: Account<'info, AllocationReceipt>,

    /// CHECK: Vault PDA yang tampung $TANI
    #[account(
        mut,
        seeds = [b"batch_vault", batch_id.as_bytes()],
        bump = batch.vault_bump,
    )]
    pub batch_vault: AccountInfo<'info>,

    #[account(
        mut,
        constraint = user_tani_account.owner == user.key(),
    )]
    pub user_tani_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = vault_tani_account.owner == batch_vault.key(),
    )]
    pub vault_tani_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub user: Signer<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn join_batch_handler(
    ctx: Context<JoinBatch>,
    batch_id: String,
    nft_quantity: u64,
) -> Result<()> {
    let batch = &mut ctx.accounts.batch;
    let clock = Clock::get()?;

    // Validasi
    require!(batch.status == BatchStatus::Open, Z4Error::BatchNotOpen);
    require!(clock.unix_timestamp < batch.end_time, Z4Error::BatchExpired);
    require!(nft_quantity > 0, Z4Error::InvalidNftQuantity);
    require!(
        batch.filled_nft + nft_quantity <= batch.total_nft,
        Z4Error::InsufficientBatchCapacity
    );

    // Hitung total TANI
    let tani_amount = batch.tani_per_nft
        .checked_mul(nft_quantity)
        .ok_or(Z4Error::Overflow)?;

    require!(
        ctx.accounts.user_tani_account.amount >= tani_amount,
        Z4Error::InsufficientTani
    );

    // Transfer $TANI dari user ke vault
    let cpi_accounts = Transfer {
        from: ctx.accounts.user_tani_account.to_account_info(),
        to: ctx.accounts.vault_tani_account.to_account_info(),
        authority: ctx.accounts.user.to_account_info(),
    };
    token::transfer(
        CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts),
        tani_amount,
    )?;

    // Update batch
    batch.filled_nft = batch.filled_nft
        .checked_add(nft_quantity)
        .ok_or(Z4Error::Overflow)?;
    batch.total_tani_collected = batch.total_tani_collected
        .checked_add(tani_amount)
        .ok_or(Z4Error::Overflow)?;

    // Auto success kalau filled >= target
    if batch.filled_nft >= batch.target_nft {
        batch.status = BatchStatus::Success;
    }

    // Save receipt on-chain
    let receipt = &mut ctx.accounts.receipt;
    let mut batch_id_bytes = [0u8; 20];
    let id_bytes = batch_id.as_bytes();
    let len = id_bytes.len().min(20);
    batch_id_bytes[..len].copy_from_slice(&id_bytes[..len]);
    receipt.batch_id = batch_id_bytes;
    receipt.owner = ctx.accounts.user.key();
    receipt.nft_quantity = nft_quantity;
    receipt.tani_amount = tani_amount;
    receipt.status = ReceiptStatus::Pending;
    receipt.created_at = clock.unix_timestamp;
    receipt.bump = ctx.bumps.receipt;

    emit!(BatchJoined {
        batch_id: batch_id.clone(),
        user: ctx.accounts.user.key(),
        nft_quantity,
        tani_amount,
        filled_nft: batch.filled_nft,
        timestamp: clock.unix_timestamp,
    });

    msg!("User join batch {}: {} NFT, {} TANI masuk vault",
        batch_id, nft_quantity, tani_amount);
    Ok(())
}

// ─── Finalize Batch ────────────────────────────────────────────────────────

#[derive(Accounts)]
#[instruction(batch_id: String)]
pub struct FinalizeBatch<'info> {
    #[account(
        mut,
        seeds = [b"batch", batch_id.as_bytes()],
        bump = batch.bump,
        has_one = authority,
    )]
    pub batch: Account<'info, BatchAccount>,

    /// CHECK: Vault PDA
    #[account(
        mut,
        seeds = [b"batch_vault", batch_id.as_bytes()],
        bump = batch.vault_bump,
    )]
    pub batch_vault: AccountInfo<'info>,

    #[account(
        seeds = [b"platform_config"],
        bump = platform_config.bump,
    )]
    pub platform_config: Account<'info, PlatformConfig>,

    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

pub fn finalize_batch_handler(
    ctx: Context<FinalizeBatch>,
    batch_id: String,
) -> Result<()> {
    let batch = &mut ctx.accounts.batch;
    let clock = Clock::get()?;

    require!(
        batch.status == BatchStatus::Open || batch.status == BatchStatus::Success,
        Z4Error::BatchAlreadyFinalized
    );

    // Tentukan hasil batch
    let is_success = batch.filled_nft >= batch.target_nft
        || clock.unix_timestamp >= batch.end_time && batch.filled_nft >= batch.target_nft;

    let is_failed = clock.unix_timestamp >= batch.end_time
        && batch.filled_nft < batch.target_nft;

    if is_success {
        batch.status = BatchStatus::Success;
        msg!("Batch {} SUKSES: {}/{} NFT terjual",
            batch_id, batch.filled_nft, batch.total_nft);
    } else if is_failed {
        batch.status = BatchStatus::Failed;
        msg!("Batch {} GAGAL: {}/{} NFT (target: {})",
            batch_id, batch.filled_nft, batch.total_nft, batch.target_nft);
    } else {
        return Err(Z4Error::BatchNotEnded.into());
    }

    emit!(BatchFinalized {
        batch_id: batch_id.clone(),
        status: if batch.status == BatchStatus::Success {
            "success".to_string()
        } else {
            "failed".to_string()
        },
        filled_nft: batch.filled_nft,
        total_tani: batch.total_tani_collected,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

// ─── Mint NFT (setelah batch sukses) ──────────────────────────────────────

#[derive(Accounts)]
#[instruction(batch_id: String)]
pub struct MintBatchNft<'info> {
    #[account(
        mut,
        seeds = [b"batch", batch_id.as_bytes()],
        bump = batch.bump,
    )]
    pub batch: Account<'info, BatchAccount>,

    #[account(
        mut,
        seeds = [b"receipt", batch_id.as_bytes(), user.key().as_ref()],
        bump = receipt.bump,
        constraint = receipt.owner == user.key(),
    )]
    pub receipt: Account<'info, AllocationReceipt>,

    /// CHECK: Vault PDA
    #[account(
        mut,
        seeds = [b"batch_vault", batch_id.as_bytes()],
        bump = batch.vault_bump,
    )]
    pub batch_vault: AccountInfo<'info>,

    #[account(
        mut,
        constraint = vault_tani_account.owner == batch_vault.key(),
    )]
    pub vault_tani_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = tani_treasury.key() == platform_config.tani_treasury,
    )]
    pub tani_treasury: Account<'info, TokenAccount>,

    /// CHECK: TANI mint untuk burn
    #[account(
        mut,
        constraint = tani_mint.key() == platform_config.tani_mint,
    )]
    pub tani_mint: AccountInfo<'info>,

    #[account(
        seeds = [b"platform_config"],
        bump = platform_config.bump,
    )]
    pub platform_config: Account<'info, PlatformConfig>,

    #[account(mut)]
    pub user: Signer<'info>,

    pub token_program: Program<'info, Token>,
}

pub fn mint_batch_nft_handler(
    ctx: Context<MintBatchNft>,
    batch_id: String,
) -> Result<()> {
    let batch = &ctx.accounts.batch;
    let receipt = &mut ctx.accounts.receipt;

    // Validasi
    require!(batch.status == BatchStatus::Success, Z4Error::BatchTargetNotMet);
    require!(receipt.status == ReceiptStatus::Pending, Z4Error::ReceiptAlreadyClaimed);

    let tani_amount = receipt.tani_amount;

    // Hitung 70/30
    let treasury_amount = tani_amount
        .checked_mul(70)
        .ok_or(Z4Error::Overflow)?
        .checked_div(100)
        .ok_or(Z4Error::Overflow)?;

    let burn_amount = tani_amount
        .checked_sub(treasury_amount)
        .ok_or(Z4Error::Overflow)?;

    // Sign dengan vault PDA
    let batch_id_bytes = batch_id.as_bytes();
    let vault_seeds = &[
        b"batch_vault".as_ref(),
        batch_id_bytes,
        &[batch.vault_bump],
    ];
    let signer_seeds = &[&vault_seeds[..]];

    // Step 1: Transfer 70% ke TANI Treasury
    let cpi_treasury = Transfer {
        from: ctx.accounts.vault_tani_account.to_account_info(),
        to: ctx.accounts.tani_treasury.to_account_info(),
        authority: ctx.accounts.batch_vault.to_account_info(),
    };
    token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_treasury,
            signer_seeds,
        ),
        treasury_amount,
    )?;

    // Step 2: Burn 30% valid SPL burn
    let cpi_burn = Burn {
        mint: ctx.accounts.tani_mint.to_account_info(),
        from: ctx.accounts.vault_tani_account.to_account_info(),
        authority: ctx.accounts.batch_vault.to_account_info(),
    };
    token::burn(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_burn,
            signer_seeds,
        ),
        burn_amount,
    )?;

    // Step 3: Mark receipt sebagai minted
    receipt.status = ReceiptStatus::Minted;

    let nft_quantity = receipt.nft_quantity;

    emit!(NftMinted {
        batch_id: batch_id.clone(),
        owner: ctx.accounts.user.key(),
        nft_quantity,
        tani_spent: tani_amount,
        treasury_amount,
        burn_amount,
        timestamp: Clock::get()?.unix_timestamp,
    });

    msg!("NFT Minted: {} NFT untuk {}, treasury={}, burn={}",
        nft_quantity,
        ctx.accounts.user.key(),
        treasury_amount,
        burn_amount
    );
    Ok(())
}

// ─── Claim Refund (batch gagal) ────────────────────────────────────────────

#[derive(Accounts)]
#[instruction(batch_id: String)]
pub struct ClaimRefund<'info> {
    #[account(
        seeds = [b"batch", batch_id.as_bytes()],
        bump = batch.bump,
    )]
    pub batch: Account<'info, BatchAccount>,

    #[account(
        mut,
        seeds = [b"receipt", batch_id.as_bytes(), user.key().as_ref()],
        bump = receipt.bump,
        constraint = receipt.owner == user.key(),
    )]
    pub receipt: Account<'info, AllocationReceipt>,

    /// CHECK: Vault PDA
    #[account(
        mut,
        seeds = [b"batch_vault", batch_id.as_bytes()],
        bump = batch.vault_bump,
    )]
    pub batch_vault: AccountInfo<'info>,

    #[account(
        mut,
        constraint = vault_tani_account.owner == batch_vault.key(),
    )]
    pub vault_tani_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = user_tani_account.owner == user.key(),
    )]
    pub user_tani_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub user: Signer<'info>,

    pub token_program: Program<'info, Token>,
}

pub fn claim_refund_handler(
    ctx: Context<ClaimRefund>,
    batch_id: String,
) -> Result<()> {
    let batch = &ctx.accounts.batch;
    let receipt = &mut ctx.accounts.receipt;

    // Validasi
    require!(batch.status == BatchStatus::Failed, Z4Error::BatchNotFailed);
    require!(receipt.status == ReceiptStatus::Pending, Z4Error::ReceiptAlreadyClaimed);

    let refund_amount = receipt.tani_amount;

    // Sign dengan vault PDA
    let batch_id_bytes = batch_id.as_bytes();
    let vault_seeds = &[
        b"batch_vault".as_ref(),
        batch_id_bytes,
        &[batch.vault_bump],
    ];
    let signer_seeds = &[&vault_seeds[..]];

    // Transfer $TANI kembali ke user
    let cpi_accounts = Transfer {
        from: ctx.accounts.vault_tani_account.to_account_info(),
        to: ctx.accounts.user_tani_account.to_account_info(),
        authority: ctx.accounts.batch_vault.to_account_info(),
    };
    token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
            signer_seeds,
        ),
        refund_amount,
    )?;

    // Mark receipt sebagai refunded
    receipt.status = ReceiptStatus::Refunded;

    emit!(RefundClaimed {
        batch_id: batch_id.clone(),
        user: ctx.accounts.user.key(),
        tani_refunded: refund_amount,
        timestamp: Clock::get()?.unix_timestamp,
    });

    msg!("Refund: {} TANI kembali ke {}", refund_amount, ctx.accounts.user.key());
    Ok(())
}
