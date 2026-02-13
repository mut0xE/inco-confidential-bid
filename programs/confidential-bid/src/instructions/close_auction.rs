use anchor_lang::prelude::*;

use crate::{
    constants::AUCTION_SEED,
    error::AuctionError,
    events::AuctionClosed,
    state::{AuctionState, AuctionStatus},
};

#[derive(Accounts)]
pub struct CloseAuction<'info> {
    #[account(mut)]
    pub organizer: Signer<'info>,

    #[account(
           mut,
           seeds=[
               AUCTION_SEED,
               organizer.key().as_ref(),
               auction.auction_id.to_le_bytes().as_ref()
           ],
           bump = auction.auction_bump,
       )]
    pub auction: Account<'info, AuctionState>,
}

impl<'info> CloseAuction<'info> {
    pub fn handler(&mut self) -> Result<()> {
        let auction = &mut self.auction;
        let current_time = Clock::get()?.unix_timestamp;

        require!(
            auction.auction_status == AuctionStatus::Open,
            AuctionError::AuctionNotOpen
        );
        require!(
            current_time >= auction.end_time,
            AuctionError::AuctionNotEnded
        );
        require!(
            self.organizer.key() == auction.organizer.key(),
            AuctionError::Unauthorized
        );
        auction.auction_status = AuctionStatus::Closed;

        emit!(AuctionClosed {
            auction_id: auction.auction_id,
            organizer: self.organizer.key(),
            timestamp: current_time,
        });
        Ok(())
    }
}
