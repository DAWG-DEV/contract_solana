use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{transfer, Mint, Token, TokenAccount, Transfer},
};
use std::mem::size_of;

declare_id!("2A6A8TKbKdP5goC1WLZCB3Wd86bzM85x3ZgNJN1EHssi");

#[program]
mod claim {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let global = &mut ctx.accounts.global;
        global.initialized = true;
        global.authority = ctx.accounts.authority.key();

        msg!("init authority {}", global.authority.to_string());
        Ok(())
    }

    pub fn set_enabled(ctx: Context<SetEnabled>, enabled: bool) -> Result<()> {
        let global = &mut ctx.accounts.global;
        global.is_enabled = enabled;
        msg!("Set enabled: {}", enabled);
        Ok(())
    }

    pub fn update_user_amount(
        ctx: Context<UpdateUserAmount>,
        user: Pubkey,
        amount: u64,
    ) -> Result<()> {
        let user_amount = &mut ctx.accounts.user_token_amount;
        user_amount.owner = user;
        user_amount.amount = amount;
        msg!("update user {} amount {}", user.to_string(), amount);
        Ok(())
    }

    pub fn claim_token(ctx: Context<ClaimToken>) -> Result<()> {
        let global = &ctx.accounts.global;
        let claim_token_account = &mut ctx.accounts.user_token_amount;
        let mint = &ctx.accounts.mint;

        // signer: global account pubKey
        let seeds = &[GLOBAL_SEED.as_bytes(), &[ctx.bumps.global]];
        let transfer_signer = [&seeds[..]];

        let user_ata = &mut ctx.accounts.user_ata;
        let program_ata = &mut ctx.accounts.program_ata;

        let amount = claim_token_account.amount;

        claim_token_account.close(ctx.accounts.authority.clone())?;
        transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: program_ata.to_account_info(),
                    to: user_ata.to_account_info(),
                    authority: global.to_account_info(),
                },
                &transfer_signer,
            ),
            amount * 10u64.pow(mint.decimals as u32),
        )?;
        msg!(
            "transfer to ata {} user {} amount {}",
            user_ata.key().to_string(),
            &ctx.accounts.signer.key().to_string(),
            amount
        );
        Ok(())
    }
}

#[constant]
pub const GLOBAL_SEED: &str = "global";
pub const CLAIM_RECORD_SEED: &str = "claim_seed";
pub const MINT_SEED: &str = "mint";

#[account]
#[derive(Default)]
pub struct Global {
    pub initialized: bool,
    pub authority: Pubkey,
    pub is_enabled: bool,
}

// The reason why not using a Map or Vec to storage users:
// will have millions of users, which will exceed the account size restriction(10M)
#[account]
#[derive(Default)]
pub struct UserTokenAmount {
    pub owner: Pubkey,
    pub amount: u64,
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    pub authority: AccountInfo<'info>,

    #[account(mut)]
    pub signer: Signer<'info>,

    #[account(
        init_if_needed,
        payer = signer,
        space = size_of::<Global>() + 8,
        seeds = [GLOBAL_SEED.as_ref()],
        bump,
        constraint = global.initialized == false @ Errors::AlreadyInitialized,
    )]
    pub global: Account<'info, Global>,

    pub mint: Account<'info, Mint>,
    #[account(
        init_if_needed,
        payer = signer,
        associated_token::mint = mint,
        associated_token::authority = global,
    )]
    pub program_ata: Account<'info, TokenAccount>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub rent: Sysvar<'info, Rent>,
}

// should only authority call this ix
#[derive(Accounts)]
pub struct SetEnabled<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,

    #[account(
        mut,
        seeds = [GLOBAL_SEED.as_ref()],
        bump,
        constraint = global.initialized == true @ Errors::NotInitialized,
        constraint = global.authority == signer.key() @ Errors::NotAuthorized,
    )]
    pub global: Account<'info, Global>,

    pub system_program: Program<'info, System>,
}

// should only authority call this ix
#[derive(Accounts)]
#[instruction(user: Pubkey)]
pub struct UpdateUserAmount<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,

    #[account(
        seeds = [GLOBAL_SEED.as_ref()],
        bump,
        constraint = global.initialized == true @ Errors::NotInitialized,
        constraint = global.authority == signer.key() @ Errors::NotAuthorized,
    )]
    pub global: Account<'info, Global>,

    #[account(
        init_if_needed,
        payer = signer,
        seeds = [CLAIM_RECORD_SEED.as_ref(), user.as_ref()],
        space = size_of::<UserTokenAmount>() + 8,
        bump,
    )]
    pub user_token_amount: Account<'info, UserTokenAmount>,

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct ClaimToken<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,

    #[account(mut)]
    pub authority: AccountInfo<'info>,

    #[account(
        seeds = [GLOBAL_SEED.as_ref()],
        bump,
        constraint = global.initialized == true @ Errors::NotInitialized,
        constraint = global.is_enabled == true @ Errors::NotEnabled,
        constraint = global.authority == authority.key() @ Errors::NotAuthorized,
    )]
    pub global: Account<'info, Global>,

    pub mint: Account<'info, Mint>,

    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = global,
    )]
    pub program_ata: Account<'info, TokenAccount>,

    #[account(
        init_if_needed,
        payer = signer,
        associated_token::mint = mint,
        associated_token::authority = signer,
    )]
    pub user_ata: Account<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [CLAIM_RECORD_SEED.as_ref(), user_token_amount.owner.as_ref()],
        bump,
        constraint = user_token_amount.owner == signer.key() @ Errors::NotAuthorized,
        constraint = user_token_amount.amount > 0 @ Errors::NotSufficientAmount,
    )]
    pub user_token_amount: Account<'info, UserTokenAmount>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub rent: Sysvar<'info, Rent>,
}

#[error_code]
pub enum Errors {
    #[msg("The given account is not authorized to execute this instruction.")]
    NotAuthorized,

    #[msg("The program is already initialized.")]
    AlreadyInitialized,

    #[msg("The program is not initialized.")]
    NotInitialized,

    #[msg("The program is not enabled.")]
    NotEnabled,

    #[msg("The mint key is not invalid")]
    NotInvalidMintKey,

    #[msg("The amount is not sufficient")]
    NotSufficientAmount,
}
