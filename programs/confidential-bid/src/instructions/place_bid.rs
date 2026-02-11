use anchor_lang::prelude::*;
use inco_lightning::{
    cpi::{as_euint128, new_euint128, Operation},
    IncoLightning, ID as INCO_LIGHTNING_ID,
};
use inco_token::cpi::{accounts::TransferChecked, transfer_checked};

use crate::{
    constants::{AUCTION_SEED, BID_SEED},
    error::AuctionError,
    state::{AuctionState, AuctionStatus, Bid},
};

#[derive(Accounts)]
pub struct PlaceBid<'info> {
    #[account(mut)]
    pub bidder: Signer<'info>,

    /// CHECK: used only for PDA seed verification
    pub organizer: UncheckedAccount<'info>,

    /// CHECK: Bidder's confidential token account
    #[account(mut)]
    pub bidder_token_ata: AccountInfo<'info>,

    /// CHECK: Inco mint used for bidding, must match auction state
    #[account(mut)]
    pub bid_mint: AccountInfo<'info>,

    /// CHECK: shared auction escrow Inco token account, must match auction state
    #[account(mut)]
    pub bid_vault: AccountInfo<'info>,

    #[account(
            init,
            payer = bidder,
            space = 8 + Bid::INIT_SPACE,
            seeds = [
                BID_SEED,
                auction.key().as_ref(),
                bidder.key().as_ref()
            ],
            bump
        )]
    pub bid: Account<'info, Bid>,

    /// - Auction state PDA
    #[account(
        mut,
        seeds=[
            AUCTION_SEED,
            organizer.key().as_ref(),
            auction.auction_id.to_le_bytes().as_ref()
        ]
        ,bump
    )]
    pub auction: Account<'info, AuctionState>,

    pub system_program: Program<'info, System>,

    /// CHECK: Inco Token program
    pub inco_token_program: AccountInfo<'info>,

    /// CHECK: Inco Lightning program
    #[account(address = INCO_LIGHTNING_ID)]
    pub inco_lightning_program: Program<'info, IncoLightning>,
}

impl<'info> PlaceBid<'info> {
    pub fn handler(
        &mut self,
        bid_amount: Vec<u8>,
        bid_timestamp: Vec<u8>,
        bump: &PlaceBidBumps,
        input_type: u8,
    ) -> Result<()> {
        require!(
            self.auction.auction_status == AuctionStatus::Open,
            AuctionError::AuctionNotOpen
        );
        require!(
            self.bid_mint.key() == self.auction.bid_token_mint.key(),
            AuctionError::InvalidBidMint
        );
        require!(
            self.bid_vault.key() == self.auction.bid_vault,
            AuctionError::InvalidBidVault
        );
        require!(!bid_amount.is_empty(), AuctionError::InvalidBidAmount);

        // Verify bid_mint is owned by Inco program
        require!(
            self.bid_mint.owner == self.inco_token_program.key,
            AuctionError::InvalidBidMint
        );
        let inco_program = self.inco_token_program.to_account_info();

        let current_time = Clock::get()?.unix_timestamp;
        let cpi_ctx = CpiContext::new(
            inco_program.clone(),
            Operation {
                signer: self.bidder.to_account_info(),
            },
        );
        let enc_time_stamp = as_euint128(cpi_ctx, current_time as u128)?;

        let cpi_ctx = CpiContext::new(
            inco_program.clone(),
            Operation {
                signer: self.bidder.to_account_info(),
            },
        );
        let enc_bid_amount = new_euint128(cpi_ctx, bid_amount.clone(), input_type)?;

        let cpi_transfer = CpiContext::new(
            inco_program.clone(),
            TransferChecked {
                source: self.bidder_token_ata.to_account_info(),
                mint: self.bid_mint.to_account_info(),
                destination: self.bid_vault.to_account_info(),
                authority: self.bidder.to_account_info(),
                inco_lightning_program: self.inco_lightning_program.to_account_info(),
                system_program: self.system_program.to_account_info(),
            },
        );
        let bid_mint_decimals =
            inco_token::IncoMint::try_deserialize(&mut &self.bid_mint.try_borrow_data()?[..])?;

        transfer_checked(
            cpi_transfer,
            bid_amount,
            input_type,
            bid_mint_decimals.decimals,
        )?;

        self.auction
            .bid_count
            .checked_add(1)
            .ok_or(AuctionError::MathOverflow)?;

        // Initialize bid account
        self.bid.set_inner(Bid {
            bidder: self.bidder.key(),
            auction: self.auction.key(),
            bid_amount: enc_bid_amount.0,
            time_stamp: enc_time_stamp.0,
            bid_bump: bump.bid,
        });

        Ok(())
    }
}
