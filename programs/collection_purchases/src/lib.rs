#![allow(unexpected_cfgs)]
const MAX_COLLECTION_SIZE: u16 = 2000;
use anchor_lang::prelude::*;
use mpl_bubblegum::types::{MetadataArgs, TokenProgramVersion, TokenStandard, Creator, Collection};
use mpl_bubblegum::instructions::MintV1InstructionArgs;
use mpl_bubblegum::instructions::MintV1CpiAccounts;
use mpl_bubblegum::instructions::MintV1Cpi;
use mpl_bubblegum::utils::get_asset_id;
use anchor_lang::solana_program::pubkey::Pubkey;
use anchor_lang::solana_program::{system_instruction, sysvar::Sysvar};

use collection_price_manager::MerkleTreeIndex;

use collection_price_manager::program::CollectionPriceManager;
//use collection_price_manager::FetchPrices;
use collection_price_manager::CollectionPrices;

declare_id!("4Cu1DNPbgnDmCMCpBrgGuGhTJMfwoeWJXqPizDNTZesU");

#[program]
pub mod collection_purchases {
    use super::*;
    
    

    pub fn mint_and_initialize_cnft(
        ctx: Context<MintAndInitializeCNFT>,
        purchase_indices: Vec<u16>,
    ) -> Result<()> {
        // Check if user already owns a cNFT for this collection
        let (existing_pda, _) = Pubkey::find_program_address(
            &[b"purchases", ctx.accounts.payer.key().as_ref()],
            &ctx.program_id,
        );
        
        if existing_pda != ctx.accounts.pda_purchases.key() {
            return Err(ErrorCode::UserAlreadyOwnsCNFT.into());
        }

        // Get Price size directly from collection prices pda
        let collection_size = ctx.accounts.collection_prices.size; // get this from collectionPDA

        // Validate collection size
        if collection_size == 0 || collection_size > MAX_COLLECTION_SIZE {
            return Err(ErrorCode::InvalidCollectionSize.into());
        }

        // Validate purchase indices
        for &index in &purchase_indices {
            if index >= collection_size {
                return Err(ErrorCode::InvalidPurchaseIndex.into());
            }
        }

        // Calculate total price
        let mut total_price: u64 = 0;
        for &index in &purchase_indices {
            total_price = total_price.checked_add(ctx.accounts.collection_prices.prices[index as usize])
                .ok_or(ErrorCode::PriceOverflow)?;
        }

        // Check if payer has enough lamports
        if ctx.accounts.payer.lamports() < total_price {
            return Err(ErrorCode::InsufficientLamports.into());
        }

        // Calculate required space and lamports
        let num_bytes = (collection_size + 7) / 8;
        let space = 8 + 32 + 4 + num_bytes as usize;
        let rent = Rent::get()?;
        let required_lamports = rent.minimum_balance(space);

        // Check if the payer has enough lamports for both rent and purchase
        if ctx.accounts.payer.lamports() < required_lamports + total_price {
            return Err(ErrorCode::InsufficientLamports.into());
        }

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
        let mint_authority = &ctx.accounts.mint_authority.to_account_info();
    
        // Define metadata for the cNFT
        let metadata = MetadataArgs {
            name: "My NFT".to_string(),
            symbol: "NFT".to_string(),
            uri: "https://example.com/nft/metadata.json".to_string(),
            seller_fee_basis_points: 500, // 5% royalty
            creators: vec![
                Creator {
                    address: ctx.accounts.mint_authority.key(), // Use mint authority as creator
                    verified: true,
                    share: 100,
                },
            ],
            primary_sale_happened: false,
            is_mutable: true,
            edition_nonce: Some(1),
            token_standard: Some(TokenStandard::NonFungible),
            collection: Some(Collection {
                verified: true, // Always set to true since we're using the collection's mint authority
                key: ctx.accounts.collection_prices.collection_address, // Use collection address from PDA
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
                tree_creator_or_delegate: mint_authority, // Use mint authority as tree creator
            },
            MintV1InstructionArgs { metadata },
        );
    
        // Sign with the mint authority PDA
        let signer_seeds = &[
            b"mint_authority",
            ctx.accounts.collection_prices.collection_address.as_ref(),
            &[ctx.bumps.mint_authority],
        ];
        cpi_mint.invoke_signed(&[signer_seeds])?;
    
        // Step 2: Derive the cNFT address
        let merkle_tree_key = ctx.accounts.collection_prices.merkle_tree;
        let leaf_index = ctx.accounts.merkle_tree_index.current_index;
        let cnft_address = get_asset_id(&merkle_tree_key, leaf_index);

        // Increment the leaf index for the next mint
        ctx.accounts.merkle_tree_index.current_index += 1;

        // Validate PDA derivation
        let (expected_pda, bump) = Pubkey::find_program_address(
            &[b"purchases", cnft_address.as_ref()],
            &ctx.program_id,
        );
        
        if expected_pda != ctx.accounts.pda_purchases.key() {
            return Err(ErrorCode::InvalidPda.into());
        }

        // Step 1: Transfer lamports
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

        // Step 2: Allocate space
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

        // ✅ Step 3: Assign program ownership
        anchor_lang::solana_program::program::invoke(
            &system_instruction::assign(
                &ctx.accounts.pda_purchases.key(),
                &ctx.program_id,
            ),
            &[
                ctx.accounts.pda_purchases.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
        )?;
        // end set dynamic space and initialization
    
        // 3: Initialize PDA with the purchased indices
        let pda_data = &mut ctx.accounts.pda_purchases;
        pda_data.owner = ctx.accounts.payer.key();
        pda_data.data = vec![0; num_bytes as usize]; // Initialize with all zeros
        
        // Set the purchased indices to true (1)
        for &index in &purchase_indices {
            let byte_index = index as usize / 8;
            let bit_index = index as usize % 8;
            pda_data.data[byte_index] |= 1 << bit_index;
        }

        Ok(())
    }
    

    // Fetch the data stored in the PDA
    pub fn fetch_data(ctx: Context<FetchData>) -> Result<Vec<u8>> {
        let pda_data = &ctx.accounts.pda_purchases;
        Ok(pda_data.data.clone()) // Return a clone of the Vec<u8>
    }

    // New function: Add a purchase (modify PDA data)
    pub fn add_purchase(ctx: Context<AddPurchase>, purchase_indices: Vec<u16>) -> Result<()> {
        // Get collection size
        let collection_size = ctx.accounts.collection_prices.size;

        // Validate purchase indices
        for &index in &purchase_indices {
            if index >= collection_size {
                return Err(ErrorCode::InvalidPurchaseIndex.into());
            }
        }

        // Calculate total price
        let mut total_price: u64 = 0;
        for &index in &purchase_indices {
            total_price = total_price.checked_add(ctx.accounts.collection_prices.prices[index as usize])
                .ok_or(ErrorCode::PriceOverflow)?;
        }

        // Check if user has enough lamports
        if ctx.accounts.user.lamports() < total_price {
            return Err(ErrorCode::InsufficientLamports.into());
        }

        // Transfer lamports from user to PDA
        anchor_lang::solana_program::program::invoke(
            &system_instruction::transfer(
                &ctx.accounts.user.key(),
                &ctx.accounts.pda_purchases.key(),
                total_price,
            ),
            &[
                ctx.accounts.user.to_account_info(),
                ctx.accounts.pda_purchases.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
        )?;

        // Update PDA data with new purchases
        for &index in &purchase_indices {
            let byte_index = index as usize / 8;
            let bit_index = index as usize % 8;
            ctx.accounts.pda_purchases.data[byte_index] |= 1 << bit_index;
        }

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

    /// CHECK: Will be manually created and assigned in instruction
    #[account(mut)]
    pub pda_purchases: Account<'info, PDAPurchases>,

    #[account(mut, seeds = [b"prices", collection_address.key().as_ref()], bump)]
    pub collection_prices: Account<'info, CollectionPrices>,

    #[account(mut, seeds = [b"tree_index", collection_prices.merkle_tree.as_ref()], bump)]
    pub merkle_tree_index: Account<'info, MerkleTreeIndex>,

    /// CHECK: PDA signer for minting, derived from collection address
    #[account(seeds = [b"mint_authority", collection_prices.collection_address.as_ref()], bump)]
    pub mint_authority: AccountInfo<'info>,

    /// CHECK: This account is only used for deriving the PDA and is not read or written to.
    pub collection_address: AccountInfo<'info>,

    pub collection_price_manager_program: Program<'info, CollectionPriceManager>,
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
    /// CHECK: This account is only used for deriving the PDA and is not read or written to.
    pub cnft_address: AccountInfo<'info>,

    #[account(mut, seeds = [b"purchases", cnft_address.key().as_ref()], bump)]
    pub pda_purchases: Account<'info, PDAPurchases>,

    #[account(mut)]
    pub user: Signer<'info>,

    #[account(mut, seeds = [b"prices", collection_address.key().as_ref()], bump)]
    pub collection_prices: Account<'info, CollectionPrices>,

    /// CHECK: This account is only used for deriving the PDA and is not read or written to.
    pub collection_address: AccountInfo<'info>,

    pub collection_price_manager_program: Program<'info, CollectionPriceManager>,

    pub system_program: Program<'info, System>,
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
    #[msg("Invalid PDA derivation.")]
    InvalidPda,
    #[msg("Invalid collection size.")]
    InvalidCollectionSize,
    #[msg("Invalid purchase index.")]
    InvalidPurchaseIndex,
    #[msg("Price overflow.")]
    PriceOverflow,
    #[msg("User already owns a cNFT for this collection.")]
    UserAlreadyOwnsCNFT,
}
