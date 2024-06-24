use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    metadata::{
        create_metadata_accounts_v3, mpl_token_metadata::types::DataV2, CreateMetadataAccountsV3,
        Metadata as Metaplex,
    },
    token::{mint_to, set_authority, Mint, MintTo, SetAuthority, Token, TokenAccount},
};
use spl_token::instruction::AuthorityType;

declare_id!("31eJYqhStBcbBchqUhJ1KQL7CC8H9HK6TVggqY14ea8y");

#[program]
pub mod dawg_token {
    use super::*;

    pub fn init_token(
        ctx: Context<InitToken>,
        token_name: String,
        token_symbol: String,
        token_uri: String,
        total_supply: u64,
    ) -> Result<()> {
        let seeds = &[MINT_SEED.as_bytes(), &[ctx.bumps.mint]];
        let signer = [&seeds[..]];

        // 1. Init token
        let token_data: DataV2 = DataV2 {
            name: token_name.clone(),
            symbol: token_symbol.clone(),
            uri: token_uri.clone(),
            seller_fee_basis_points: 0,
            creators: None,
            collection: None,
            uses: None,
        };

        let metadata_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_metadata_program.to_account_info(),
            CreateMetadataAccountsV3 {
                payer: ctx.accounts.payer.to_account_info(),
                update_authority: ctx.accounts.mint.to_account_info(),
                mint: ctx.accounts.mint.to_account_info(),
                metadata: ctx.accounts.metadata.to_account_info(),
                mint_authority: ctx.accounts.mint.to_account_info(),
                system_program: ctx.accounts.system_program.to_account_info(),
                rent: ctx.accounts.rent.to_account_info(),
            },
            &signer,
        );

        create_metadata_accounts_v3(metadata_ctx, token_data, false, true, None)?;
        msg!(
            "Token init successfully name {} symble {} total supply {}.",
            token_name,
            token_symbol,
            total_supply
        );

        // 2. Mint token
        mint_to(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                MintTo {
                    authority: ctx.accounts.mint.to_account_info(),
                    to: ctx.accounts.destination.to_account_info(),
                    mint: ctx.accounts.mint.to_account_info(),
                },
                &signer,
            ),
            total_supply * 10u64.pow(5),
        )?;
        msg!("Token minted successfully.");

        // 3. Revoke mint authority
        set_authority(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                SetAuthority {
                    current_authority: ctx.accounts.mint.to_account_info(),
                    account_or_mint: ctx.accounts.mint.to_account_info(),
                },
                &signer,
            ),
            AuthorityType::MintTokens,
            None,
        )?;
        msg!("Revoke minted authority successfully.");

        Ok(())
    }
}

#[constant]
pub const MINT_SEED: &str = "mint";

#[derive(Accounts)]
pub struct InitToken<'info> {
    #[account(mut)]
    pub metadata: UncheckedAccount<'info>,
    #[account(
        init,
        seeds = [MINT_SEED.as_bytes()],
        bump,
        payer = payer,
        mint::decimals = 5,
        mint::authority = mint,
    )]
    pub mint: Account<'info, Mint>,

    #[account(
        init_if_needed,
        payer = payer,
        associated_token::mint = mint,
        associated_token::authority = master,
    )]
    pub destination: Account<'info, TokenAccount>,

    pub master: AccountInfo<'info>,
    #[account(mut)]
    pub payer: Signer<'info>,

    pub rent: Sysvar<'info, Rent>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_metadata_program: Program<'info, Metaplex>,
}
