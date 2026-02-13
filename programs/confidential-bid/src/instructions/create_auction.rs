use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked},
};
use inco_lightning::{IncoLightning, ID as INCO_LIGHTNING_ID};
use inco_token::cpi::{accounts::CreateIdempotent, create_idempotent};

use crate::{
    constants::AUCTION_SEED,
    error::AuctionError,
    events::AuctionCreated,
    state::{AuctionState, AuctionStatus, AuctionType},
};

#[derive(Accounts)]
#[instruction(auction_id:u64)]
pub struct CreateAuction<'info> {
    /// The auction creator/organizer who owns the token to be auctioned
    #[account(mut)]
    pub organizer: Signer<'info>,

    /// The NFT mint being auctioned
    #[account(mint::token_program=token_program)]
    pub mint: InterfaceAccount<'info, Mint>,

    /// CHECK: BIDDING TOKEN INCO MINT - The token used for placing bids
    pub bid_token_mint: AccountInfo<'info>,

    /// CHECK: auction bid Inco vault
    #[account(mut)]
    pub bid_vault: AccountInfo<'info>,

    /// - Vault token account that holds Bidding NFT.
    /// - Owned by the Auction PDA
    /// - Created as ATA for deterministic address derivation
    #[account(
        init,
        payer=organizer,
        associated_token::mint = mint,
        associated_token::authority = auction,
        associated_token::token_program = token_program,
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,

    /// - Auction state PDA storing metadata.
    /// - Stores all auction metadata
    /// - Derived from [AUCTION_SEED, organizer, auction_id]
    #[account(
        init,
        payer=organizer,
        space=8 + AuctionState::LEN,
        seeds=[
            AUCTION_SEED,
            organizer.key().as_ref(),
            auction_id.to_le_bytes().as_ref()
        ]
        ,bump
    )]
    pub auction: Account<'info, AuctionState>,

    /// Organizer's token account holding the NFT to be escrowed
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = organizer,
        associated_token::token_program = token_program
    )]
    pub organizer_token_account: InterfaceAccount<'info, TokenAccount>,

    pub system_program: Program<'info, System>,

    /// Associated Token Program for creating and managing ATAs.
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Interface<'info, TokenInterface>,

    /// CHECK: Inco Token program
    pub inco_token_program: AccountInfo<'info>,

    /// CHECK: Inco Lightning program
    #[account(address = INCO_LIGHTNING_ID)]
    pub inco_lightning_program: Program<'info, IncoLightning>,
}

impl<'info> CreateAuction<'info> {
    pub fn handler(
        &mut self,
        auction_id: u64,
        start_time: i64,
        end_time: i64,
        reserve_price: u64,
        auction_type: AuctionType,
        token_amount: u64,
        bump: &CreateAuctionBumps,
    ) -> Result<()> {
        // validate that bid token mint is from Inco token program
        require!(
            self.bid_token_mint.owner == self.inco_token_program.key,
            AuctionError::InvalidBidMint
        );

        // Validate auction timing
        let current_time = Clock::get()?.unix_timestamp;
        require!(start_time > current_time, AuctionError::InvalidStartTime);
        require!(end_time > start_time, AuctionError::InvalidEndTime);

        // Validate token amount
        require!(token_amount > 0, AuctionError::InvalidTokenAmount);

        // Check organizer has sufficient balance
        require!(
            self.organizer_token_account.amount >= token_amount,
            AuctionError::InsufficientBalance
        );

        // Transfer tokens from organizer to vault
        let cpi_ctx = CpiContext::new(
            self.token_program.to_account_info(),
            TransferChecked {
                from: self.organizer_token_account.to_account_info(),
                mint: self.mint.to_account_info(),
                to: self.vault.to_account_info(),
                authority: self.organizer.to_account_info(),
            },
        );
        transfer_checked(cpi_ctx, token_amount, self.mint.decimals)?;

        // CREATE BID VAULT (ATA)
        let ctx_accounts = CreateIdempotent {
            payer: self.organizer.to_account_info(),
            associated_token: self.bid_vault.to_account_info(),
            wallet: self.auction.to_account_info(),
            mint: self.bid_token_mint.to_account_info(),
            system_program: self.system_program.to_account_info(),
            inco_lightning_program: self.inco_lightning_program.to_account_info(),
        };
        create_idempotent(CpiContext::new(
            self.inco_token_program.to_account_info(),
            ctx_accounts,
        ))?;

        // Initialize auction state
        self.auction.set_inner(AuctionState {
            organizer: self.organizer.key(),
            mint: self.mint.key(),
            highest_bidder: None,
            vault: self.vault.key(),
            highest_bid: 0,
            second_highest_bid: None,
            highest_timestamp: 0,
            start_time,
            end_time,
            reserve_price,
            auction_id,
            bid_count: 0,
            auction_status: AuctionStatus::Open,
            auction_type,
            auction_bump: bump.auction,
            bid_token_mint: self.bid_token_mint.key(),
            bid_vault: self.bid_vault.key(),
        });

        emit!(AuctionCreated {
            auction_id,
            organizer: self.organizer.key(),
            mint: self.mint.key(),
            amount: token_amount,
            decimals: self.mint.decimals,
            start_time,
            end_time,
            reserve_price,
            auction_type: auction_type,
            bid_token_mint: self.bid_token_mint.key()
        });

        Ok(())
    }
}
