use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, TokenAccount, Token, MintTo};
use mpl_token_metadata::instructions::create_metadata_account_v3;
use solana_program::program::invoke_signed;
use spl_associated_token_account::ID as associated_token_program;

declare_id!("4Cu1DNPbgnDmCMCpBrgGuGhTJMfwoeWJXqPizDNTZesU");

#[program]
pub mod new_test_App {
    use super::*;

    // Function to purchase items
    pub fn purchase_items(ctx: Context<PurchaseItems>, items: Vec<u8>) -> Result<()> {
        let user = &ctx.accounts.user;
        let pda_data = &mut ctx.accounts.pda_purchases;

        // Step 1: If no cNFT exists, mint a new one
        if ctx.accounts.user_cnft.is_none() {
            mint_cnft(&ctx)?;
        }

        // Step 2: Store purchased items as a bitmask (100 bytes)
        let max_length = pda_data.data.len().min(items.len());
        pda_data.data[..max_length].copy_from_slice(&items[..max_length]);

        Ok(())
    }
}

// 🟢 Function to mint a cNFT
fn mint_cnft(ctx: &Context<PurchaseItems>) -> Result<()> {
    let cNFT_mint = &ctx.accounts.cnft_mint;
    let user_cnft_token_account = &ctx.accounts.user_cnft_token_account;
    let system_program = &ctx.accounts.system_program;
    let token_program = &ctx.accounts.token_program;
    let rent = &ctx.accounts.rent;
    
    let mint_seeds: &[&[u8]] = &[b"cnft", ctx.accounts.user.key().as_ref()];
    let (mint_pda, mint_bump) = Pubkey::find_program_address(mint_seeds, ctx.program_id);

    // Create the metadata account for the cNFT
    invoke_signed(
        &create_metadata_account_v3(
            mpl_token_metadata::ID,
            mint_pda,
            cNFT_mint.key(),
            ctx.accounts.user.key(),
            ctx.accounts.user.key(),
            ctx.accounts.user.key(),
            "User cNFT".to_string(),
            "UCNFT".to_string(),
            "".to_string(),
            None, // Collection
            None, // Uses
            0, // Seller fee basis points
            true, // Is mutable
            true, // Update authority is signer
            None, // Collection details
        ),
        &[
            ctx.accounts.user.to_account_info(),
            cNFT_mint.to_account_info(),
            system_program.to_account_info(),
            token_program.to_account_info(),
            rent.to_account_info(),
        ],
        &[&[b"cnft", ctx.accounts.user.key().as_ref(), &[mint_bump]]],
    )?;

    // Mint the NFT to the user's wallet
    invoke_signed(
        &MintTo {
            mint: cNFT_mint.to_account_info().key(),
            to: user_cnft_token_account.to_account_info().key(),
            authority: ctx.accounts.user.to_account_info().key(),
        }
        .to_account_metas(None),
        &[
            cNFT_mint.to_account_info(),
            user_cnft_token_account.to_account_info(),
            ctx.accounts.user.to_account_info(),
            token_program.to_account_info(),
        ],
        &[&[b"cnft", ctx.accounts.user.key().as_ref(), &[mint_bump]]],
    )?;

    Ok(())
}

// 🟢 Context for purchase items
#[derive(Accounts)]
pub struct PurchaseItems<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        init_if_needed,
        payer = user,
        space = 8 + 32 + 100, // Discriminator + owner + 100 bytes
        seeds = [b"purchases", user.key().as_ref()], 
        bump
    )]
    pub pda_purchases: Account<'info, PDAPurchases>,

    #[account(
        init_if_needed,
        payer = user,
        mint::decimals = 0,
        mint::authority = user,
        mint::freeze_authority = user,
        seeds = [b"cnft", user.key().as_ref()],
        bump
    )]
    pub cnft_mint: Account<'info, Mint>,

    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = cnft_mint,
        associated_token::authority = user
    )]
    pub user_cnft_token_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub user_cnft: Option<Account<'info, TokenAccount>>, // Optional, if exists

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

// 🟢 PDA storage structure
#[account]
pub struct PDAPurchases {
    pub owner: Pubkey,
    pub data: [u8; 100], // Store purchased item IDs as bitmask
}