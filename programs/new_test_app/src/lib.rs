use anchor_lang::prelude::*;
use mpl_bubblegum::types::{MetadataArgs, TokenProgramVersion, TokenStandard, Creator, Collection};
use mpl_bubblegum::instructions::MintV1InstructionArgs;
use mpl_bubblegum::instructions::MintV1CpiAccounts;
use mpl_bubblegum::instructions::MintV1Cpi;
use anchor_lang::solana_program::pubkey::Pubkey;


declare_id!("4Cu1DNPbgnDmCMCpBrgGuGhTJMfwoeWJXqPizDNTZesU");

#[program]
pub mod new_test_App {
    use super::*;
    

    // Initialize the PDA with an owner (CNFT Address)
    pub fn initialize(ctx: Context<Initialize>, cnft_address: Pubkey) -> Result<()> {
        let pda_data = &mut ctx.accounts.pda_purchases;
        pda_data.owner = cnft_address;
        pda_data.data = [0; 100]; // Allocate 100 bytes
        Ok(())
    }

    // Fetch the data stored in the PDA
    pub fn fetch_data(ctx: Context<FetchData>) -> Result<[u8; 100]> {
        let pda_data = &ctx.accounts.pda_purchases;
        Ok(pda_data.data)
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

fn mint_cnft(ctx: Context<MintCNFT>, collection_key: Pubkey, collection_verified: bool) -> Result<()> {
    let payer = &ctx.accounts.payer.to_account_info();
    let tree_config = &ctx.accounts.tree_config.to_account_info();
    let leaf_owner = &ctx.accounts.leaf_owner.to_account_info();
    let leaf_delegate = &ctx.accounts.leaf_delegate.to_account_info();
    let merkle_tree = &ctx.accounts.merkle_tree.to_account_info();
    let system_program = &ctx.accounts.system_program.to_account_info();
    let log_wrapper = &ctx.accounts.log_wrapper.to_account_info();
    let compression_program = &ctx.accounts.compression_program.to_account_info();
    let bubblegum_program = &ctx.accounts.bubblegum_program.to_account_info();

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
        token_program_version: TokenProgramVersion::Original
    };

    let cpi_mint = MintV1Cpi::new(
        bubblegum_program,
        MintV1CpiAccounts {
            compression_program: compression_program,
            leaf_delegate: leaf_delegate,
            leaf_owner: leaf_owner,
            log_wrapper: log_wrapper,
            merkle_tree: merkle_tree,
            payer: payer,
            system_program: system_program,
            tree_config: tree_config,
            tree_creator_or_delegate: leaf_delegate,
        },
        MintV1InstructionArgs { metadata },
    );

    cpi_mint.invoke();

    Ok(())
}

// Initialize PDA with the CNFT address
#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        payer = user,
        space = 8 + 32 + 100, // Discriminator + owner (32 bytes) + data (100 bytes)
        seeds = [b"purchases", cnft_address.key().as_ref()], 
        bump
    )]
    //#[account(address = BUBBLEGUM_PROGRAM_ID)] // Use the Bubblegum program ID

    pub pda_purchases: Account<'info, PDAPurchases>,
    
    #[account(mut)]
    pub user: Signer<'info>,

    pub system_program: Program<'info, System>,

    /// CHECK: This is the cNFT address the PDA is linked to, no validation needed.
    pub cnft_address: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct MintCNFT<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    /// CHECK: Verified via CPI
    pub tree_config: AccountInfo<'info>,
    
    /// CHECK: Verified via CPI
    pub leaf_owner: AccountInfo<'info>,
    
    /// CHECK: Verified via CPI
    pub leaf_delegate: AccountInfo<'info>,
    
    /// CHECK: Verified via CPI
    pub merkle_tree: AccountInfo<'info>,
    
    pub system_program: Program<'info, System>,

    /// CHECK: Log wrapper program
    #[account(address = spl_noop::id())]
    pub log_wrapper: AccountInfo<'info>,

    /// CHECK: Compression program
    #[account(address = mpl_bubblegum::ID)]
    pub compression_program: AccountInfo<'info>,

    /// CHECK: Bubblegum program
    #[account(address = mpl_bubblegum::ID)]
    pub bubblegum_program: AccountInfo<'info>,
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
    pub data: [u8; 100],
}
