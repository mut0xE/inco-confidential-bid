use anchor_lang::prelude::*;
use inco_lightning::{
    cpi::{as_euint128, e_ge, Operation},
    Ebool, Euint128, IncoLightning, ID as INCO_LIGHTNING_ID,
};

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

    pub system_program: Program<'info, System>,

    #[account(address = INCO_LIGHTNING_ID)]
    pub inco_lightning_program: Program<'info, IncoLightning>,
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

        let inco_program = self.inco_lightning_program.to_account_info();
        let signer = self.organizer.to_account_info();

        let enc_reserve = as_euint128(
            CpiContext::new(
                inco_program.clone(),
                Operation {
                    signer: signer.clone(),
                },
            ),
            auction.reserve_price as u128,
        )?;

        // highest_bid >= reserve_price
        let reserve_met: Ebool = e_ge(
            CpiContext::new(inco_program.clone(), Operation { signer }),
            Euint128(auction.highest_bid),
            enc_reserve,
            0u8,
        )?;

        auction.reserve_met_handle = reserve_met.0;
        auction.auction_status = AuctionStatus::Closed;

        emit!(AuctionClosed {
            auction_id: auction.auction_id,
            organizer: self.organizer.key(),
            timestamp: current_time,
        });

        Ok(())
    }
}
