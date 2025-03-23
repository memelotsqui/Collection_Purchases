#![allow(unexpected_cfgs)]
const MAX_COLLECTION_SIZE: i32 = 2000;
use anchor_lang::prelude::*;
use mpl_bubblegum::types::{MetadataArgs, TokenProgramVersion, TokenStandard, Creator, Collection};
use mpl_bubblegum::instructions::MintV1InstructionArgs;
use mpl_bubblegum::instructions::MintV1CpiAccounts;
use mpl_bubblegum::instructions::MintV1Cpi;
use mpl_bubblegum::utils::get_asset_id;
use anchor_lang::solana_program::pubkey::Pubkey;
use anchor_lang::solana_program::{system_instruction, sysvar::Sysvar};

declare_id!("4Cu1DNPbgnDmCMCpBrgGuGhTJMfwoeWJXqPizDNTZesU");

#[program]
pub mod collection_purchases {
    use super::*;
    

    pub fn mint_and_initialize_cnft(
        ctx: Context<MintAndInitializeCNFT>,
        collection_key: Pubkey,
        collection_verified: bool,
    ) -> Result<()> {

        let collection_size = 40;// get this from collectionPDA

        // set dynamic space
        
        // Validate collection size
        if collection_size == 0 || collection_size > MAX_COLLECTION_SIZE {
            return Err(ErrorCode::InvalidCollectionSize.into());
        }

        // Calculate required space and lamports
        
        let num_bytes = (collection_size + 7) / 8;
        let space = 8 + 32 + 4 + num_bytes as usize;
        let rent = Rent::get()?;
        let required_lamports = rent.minimum_balance(space);

         // Check if the payer has enough lamports
        if ctx.accounts.payer.lamports() < required_lamports {
            return Err(ErrorCode::InsufficientLamports.into());
        }

        // Validate PDA derivation
        let (expected_pda, bump) = Pubkey::find_program_address(
            &[b"purchases", ctx.accounts.collection_address.key().as_ref()],
            &ctx.program_id,
        );
        if expected_pda != ctx.accounts.pda_purchases.key() {
            return Err(ErrorCode::InvalidPda.into());
        }

        // Allocate space
        anchor_lang::solana_program::program::invoke(
            &system_instruction::allocate(
                &ctx.accounts.pda_purchases.key(),
                space as u64,
            ),
            &[
                ctx.accounts.pda_purchases.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
        )?;

        // Transfer lamports
        anchor_lang::solana_program::program::invoke(
            &system_instruction::transfer(
                &ctx.accounts.payer.key(),
                &ctx.accounts.pda_purchases.key(),
                required_lamports,
            ),
            &[
                ctx.accounts.payer.to_account_info(),
                ctx.accounts.pda_purchases.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
        )?;
        // end set dynamic space



        // Step 1: Mint cNFT using the accounts from ctx
        let payer = &ctx.accounts.payer.to_account_info();
        let tree_config = &ctx.accounts.tree_config.to_account_info();
        let leaf_owner = &ctx.accounts.leaf_owner.to_account_info();
        let leaf_delegate = &ctx.accounts.leaf_delegate.to_account_info();
        let merkle_tree = &ctx.accounts.merkle_tree.to_account_info();
        let system_program = &ctx.accounts.system_program.to_account_info();
        let log_wrapper = &ctx.accounts.log_wrapper.to_account_info();
        let compression_program = &ctx.accounts.compression_program.to_account_info();
        let bubblegum_program = &ctx.accounts.bubblegum_program.to_account_info();
    
        // Define metadata for the cNFT
        let metadata = MetadataArgs {
            name: "My NFT".to_string(),
            symbol: "NFT".to_string(),
            uri: "https://example.com/nft/metadata.json".to_string(),
            seller_fee_basis_points: 500, // 5% royalty
            creators: vec![
                Creator {
                    address: ctx.accounts.payer.key(),
                    verified: true,
                    share: 100,
                },
            ],
            primary_sale_happened: false,
            is_mutable: true,
            edition_nonce: Some(1),
            token_standard: Some(TokenStandard::NonFungible),
            collection: Some(Collection {
                verified: collection_verified,
                key: collection_key,
            }),
            uses: None,
            token_program_version: TokenProgramVersion::Original,
        };
    
        // Mint the cNFT
        let cpi_mint = MintV1Cpi::new(
            bubblegum_program,
            MintV1CpiAccounts {
                compression_program,
                leaf_delegate,
                leaf_owner,
                log_wrapper,
                merkle_tree,
                payer,
                system_program,
                tree_config,
                tree_creator_or_delegate: leaf_delegate,
            },
            MintV1InstructionArgs { metadata },
        );
    
        cpi_mint.invoke()?;
    
        // Step 2: Derive the cNFT address
        let merkle_tree_key = merkle_tree.key();
        let leaf_index = 0; // Replace with the actual leaf index of the newly minted cNFT
        let cnft_address = get_asset_id(&merkle_tree_key, leaf_index);
    
        // Step 3: Initialize PDA with the new cNFT address
        let pda_data = &mut ctx.accounts.pda_purchases;
        pda_data.owner = cnft_address; // Use the cNFT address as the owner
        pda_data.data = vec![0; num_bytes as usize]; // Initialize the bitmask with zeros
    
        Ok(())
    }
    

    // Fetch the data stored in the PDA
    pub fn fetch_data(ctx: Context<FetchData>) -> Result<Vec<u8>> {
        let pda_data = &ctx.accounts.pda_purchases;
        Ok(pda_data.data.clone()) // Return a clone of the Vec<u8>
    }

    // New function: Add a purchase (modify PDA data)
    pub fn add_purchase(ctx: Context<AddPurchase>, data: Vec<u8>) -> Result<()> {
        let pda_data = &mut ctx.accounts.pda_purchases;

        // Ensure data does not exceed storage limit (100 bytes)
        let max_length = pda_data.data.len().min(data.len());
        pda_data.data[..max_length].copy_from_slice(&data[..max_length]);

        Ok(())
    }
}

#[derive(Accounts)]
pub struct MintAndInitializeCNFT<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    
    /// CHECK: Verified via CPI
    pub tree_config: AccountInfo<'info>,
    
    /// CHECK: Verified via CPI
    pub leaf_owner: AccountInfo<'info>, // This will be the NFT address
    
    /// CHECK: Verified via CPI
    pub leaf_delegate: AccountInfo<'info>,
    
    /// CHECK: Verified via CPI
    pub merkle_tree: AccountInfo<'info>,
    
    pub system_program: Program<'info, System>,
    
    /// CHECK: Log wrapper program, no validation needed as it's a known program.
    #[account(address = spl_noop::id())]
    pub log_wrapper: AccountInfo<'info>,
    
    /// CHECK: Compression program
    #[account(address = mpl_bubblegum::ID)]
    pub compression_program: AccountInfo<'info>,
    
    /// CHECK: Bubblegum program
    #[account(address = mpl_bubblegum::ID)]
    pub bubblegum_program: AccountInfo<'info>,

    // PDA Initialization
    #[account(
        init,
        payer = payer,
        space = 8 + 32 + 4 + 100,//num_bytes,
        seeds = [b"purchases", leaf_owner.key().as_ref()],
        bump
    )]
    pub pda_purchases: Account<'info, PDAPurchases>,

    /// CHECK: This account is only used for deriving the PDA and is not read or written to.
    pub collection_address: AccountInfo<'info>,
}

// Fetch PDA data
#[derive(Accounts)]
pub struct FetchData<'info> {
    #[account(seeds = [b"purchases", pda_purchases.owner.as_ref()], bump)]
    pub pda_purchases: Account<'info, PDAPurchases>,
}

// New: Modify PDA data (Add Purchase)
#[derive(Accounts)]
pub struct AddPurchase<'info> {
    #[account(mut, seeds = [b"purchases", pda_purchases.owner.as_ref()], bump)]
    pub pda_purchases: Account<'info, PDAPurchases>,

    #[account(mut)]
    pub user: Signer<'info>,
}

// PDA storage structure
#[account]
pub struct PDAPurchases {
    pub owner: Pubkey,
    pub data: Vec<u8>,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Insufficient lamports.")]
    InsufficientLamports,
    // #[msg("Account already initialized.")]
    // AccountAlreadyInitialized,
    #[msg("Invalid PDA derivation.")]
    InvalidPda,
    #[msg("Invalid collection size.")]
    InvalidCollectionSize,
    // #[msg("Data overflow.")]
    // DataOverflow,
    // #[msg("Unauthorized access.")]
    // Unauthorized,
    // #[msg("Invalid system program.")]
    // InvalidSystemProgram,
    // #[msg("Invalid account owner.")]
    // InvalidAccountOwner,
}
