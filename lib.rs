use anchor_lang::prelude::*;
use anchor_spl::token::Token;
use mpl_bubblegum::instructions::MintV1CpiBuilder;
use mpl_bubblegum::types::MetadataArgs;

declare_id!("4Cu1DNPbgnDmCMCpBrgGuGhTJMfwoeWJXqPizDNTZesU");

#[program]
pub mod new_test_App {
    use super::*;

    // Function to mint a cNFT
    pub fn mint_cnft(ctx: Context<MintCnft>, collection_name: String, space:u32) -> Result<()> {
        let user = &ctx.accounts.user;
        let merkle_tree = &ctx.accounts.merkle_tree;

        // Construct the metadata
        let metadata = MetadataArgs {
            name: collection_name,
            uri: "https://smartbuild.nyc3.cdn.digitaloceanspaces.com/tests/triangleBrace.json".to_string(),
            symbol: "".to_string(),
            seller_fee_basis_points: 0,
            creators: vec![
                mpl_bubblegum::types::Creator {
                    address: user.key(),
                    verified: false,
                    share: 100,
                },
            ],
            primary_sale_happened: false,
            is_mutable: true,
            edition_nonce: None,
            token_standard: None,
            collection: None,
            uses: None,
            token_program_version: mpl_bubblegum::types::TokenProgramVersion::Original, // Add token program version
        };

        // Create the CPI instruction using MintV1CpiBuilder
        let cpi_mint = MintV1CpiBuilder::new(&ctx.accounts.bubblegum_program.to_account_info())
            .tree_config(&ctx.accounts.tree_config.to_account_info())
            .leaf_owner(&user.to_account_info())
            .leaf_delegate(&user.to_account_info())
            .merkle_tree(&merkle_tree.to_account_info())
            .payer(&user.to_account_info())
            .tree_creator_or_delegate(&user.to_account_info())
            .log_wrapper(&ctx.accounts.log_wrapper.to_account_info())
            .compression_program(&ctx.accounts.compression_program.to_account_info())
            .system_program(&ctx.accounts.system_program.to_account_info())
            .metadata(metadata);

        // Perform the CPI
        cpi_mint.invoke_signed(&[&[b"cnft", user.key().as_ref(), &[ctx.bumps.merkle_tree]]])?;

        Ok(())
    }
}

// 🟢 Context for minting a cNFT
#[derive(Accounts)]
#[instruction(space: u32)]
pub struct MintCnft<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        init_if_needed,
        payer = user,
        space = 8 + 32 + space.as_ref(), // Discriminator + owner + 100 bytes
        seeds = [
            b"merkle_tree", user.key().as_ref()
            ], 
        bump
    )]
    pub merkle_tree: Account<'info, MerkleTree>,

    #[account(mut)]
    pub bubblegum_program: Program<'info, mpl_bubblegum>,

    #[account(mut)]
    pub tree_config: AccountInfo<'info>,

    #[account(mut)]
    pub log_wrapper: AccountInfo<'info>,

    #[account(mut)]
    pub compression_program: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
    pub token_program: Program<'info, Token>,
   
}

// 🟢 Merkle tree account
#[account]
pub struct MerkleTree {
    pub owner: Pubkey,
    pub data: [u8; 100], // Store Merkle tree data
}