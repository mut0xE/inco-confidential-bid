use anchor_lang::prelude::*;
use inco_lightning::{
    cpi::{allow, e_and, e_eq, Allow, Operation},
    Ebool, Euint128, IncoLightning, ID as INCO_LIGHTNING_ID,
};

use crate::{
    constants::{AUCTION_SEED, BID_SEED},
    error::AuctionError,
    state::{AuctionState, AuctionStatus, Bid},
};

#[derive(Accounts)]
pub struct CheckWinner<'info> {
    #[account(mut)]
    pub bidder: Signer<'info>,

    #[account(
        mut,
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
            auction.organizer.key().as_ref(),
            auction.auction_id.to_le_bytes().as_ref()
        ]
        ,bump
    )]
    pub auction: Account<'info, AuctionState>,

    /// CHECK: Inco Token program
    pub inco_token_program: AccountInfo<'info>,

    pub system_program: Program<'info, System>,

    /// CHECK: Inco Lightning program
    #[account(address = INCO_LIGHTNING_ID)]
    pub inco_lightning_program: Program<'info, IncoLightning>,
}

impl<'info> CheckWinner<'info> {
    pub fn handler(
        &mut self,
        input_type: u8,
        remaining_accounts: &[AccountInfo<'info>],
    ) -> Result<()> {
        let auction = &self.auction;
        let bid = &mut self.bid;
        let inco = self.inco_lightning_program.to_account_info();

        let signer = self.bidder.to_account_info();
        // Auction must be closed
        require!(
            auction.auction_status == AuctionStatus::Closed,
            AuctionError::AuctionNotClosed
        );

        // encrypted  bid_amount == highest_bid

        let is_highest: Ebool = e_eq(
            CpiContext::new(
                inco.clone(),
                Operation {
                    signer: signer.clone(),
                },
            ),
            Euint128(bid.bid_amount),
            Euint128(self.auction.highest_bid),
            input_type,
        )?;
        // encrypted  timestamp == highest_timestamp
        let is_earliest: Ebool = e_eq(
            CpiContext::new(
                inco.clone(),
                Operation {
                    signer: signer.clone(),
                },
            ),
            Euint128(bid.time_stamp),
            Euint128(auction.highest_timestamp),
            input_type,
        )?;

        // Convert bool handles to Euint128
        let is_highest_u = Euint128(is_highest.0);
        let is_earliest_u = Euint128(is_earliest.0);
        let reserve_met_u = Euint128(self.auction.reserve_met_handle);

        let highest_and_earliest: Euint128 = e_and(
            CpiContext::new(
                inco.clone(),
                Operation {
                    signer: signer.clone(),
                },
            ),
            is_highest_u,
            is_earliest_u,
            input_type,
        )?;

        let is_winner = e_and(
            CpiContext::new(
                inco.clone(),
                Operation {
                    signer: signer.clone(),
                },
            ),
            highest_and_earliest,
            reserve_met_u,
            0,
        )?;

        self.bid.is_winner_handle = is_winner.0;

        if remaining_accounts.len() >= 2 {
            let allow_ctx = CpiContext::new(
                inco.clone(),
                Allow {
                    allowance_account: remaining_accounts[0].clone(),
                    signer: self.bidder.to_account_info(),
                    allowed_address: remaining_accounts[1].clone(),
                    system_program: self.system_program.to_account_info(),
                },
            );
            allow(allow_ctx, is_winner.0, true, self.bidder.key())?;
        }

        msg!("Winner check completed");
        msg!("Winner handle: {}", is_winner.0);
        Ok(())
    }
}
