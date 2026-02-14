use anchor_lang::prelude::*;
use inco_lightning::{
    cpi::{allow, as_euint128, e_gt, e_select, new_euint128, Allow, Operation},
    Ebool, Euint128, IncoLightning, ID as INCO_LIGHTNING_ID,
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
        bump: &PlaceBidBumps,
        input_type: u8,
        remaining_accounts: &[AccountInfo<'info>],
    ) -> Result<()> {
        let now = Clock::get()?.unix_timestamp;
        require!(
            now >= self.auction.start_time,
            AuctionError::AuctionNotStarted
        );
        require!(now < self.auction.end_time, AuctionError::AuctionEnded);

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
            *self.bid_mint.owner == self.inco_token_program.key(),
            AuctionError::InvalidBidMint
        );

        let inco_program = self.inco_lightning_program.to_account_info();

        let current_time = Clock::get()?.unix_timestamp;
        let cpi_ctx = CpiContext::new(
            inco_program.clone(),
            Operation {
                signer: self.bidder.to_account_info(),
            },
        );
        let enc_time_stamp = as_euint128(cpi_ctx, current_time as u128)?;

        let enc_bid_amount = new_euint128(
            CpiContext::new(
                inco_program.clone(),
                Operation {
                    signer: self.bidder.to_account_info(),
                },
            ),
            bid_amount.clone(),
            input_type,
        )?;

        let cpi_transfer = CpiContext::new(
            self.inco_token_program.to_account_info().clone(),
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

        self.auction.bid_count = self
            .auction
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
            is_winner_handle: 0,
            claimed: false,
        });

        let previous_highest_bid = Euint128(self.auction.highest_bid);

        let previous_second = Euint128(
            self.auction
                .second_highest_bid
                .unwrap_or(self.auction.highest_bid),
        );

        // Check if new bid is greater than current highest
        // does new bid higher than current highest
        let is_gt_highest: Ebool = e_gt(
            CpiContext::new(
                inco_program.clone(),
                Operation {
                    signer: self.bidder.to_account_info(),
                },
            ),
            enc_bid_amount,
            previous_highest_bid,
            input_type,
        )?;

        let cpi_ctx = CpiContext::new(
            inco_program.clone(),
            Operation {
                signer: self.bidder.to_account_info(),
            },
        );

        // If new bid is greater -> highest becomes new bid
        // Otherwise ->  keep previous highest
        let e_new_highest: Euint128 = e_select(
            cpi_ctx,
            is_gt_highest,
            enc_bid_amount,
            previous_highest_bid,
            input_type,
        )?;

        //  Check if new bid is greater than previous second
        let is_gt_second: Ebool = e_gt(
            CpiContext::new(
                inco_program.clone(),
                Operation {
                    signer: self.bidder.to_account_info(),
                },
            ),
            enc_bid_amount,
            previous_second,
            input_type,
        )?;

        let temp_second: Euint128 = e_select(
            CpiContext::new(
                inco_program.clone(),
                Operation {
                    signer: self.bidder.to_account_info(),
                },
            ),
            is_gt_second,
            enc_bid_amount,
            previous_second,
            input_type,
        )?;

        let cpi_ctx = CpiContext::new(
            inco_program.clone(),
            Operation {
                signer: self.bidder.to_account_info(),
            },
        );

        // If new bid is greater than second -> use new bid
        // Otherwise -> keep previous second
        let new_second = e_select(
            cpi_ctx,
            is_gt_highest,
            previous_highest_bid, // prev highest becomes second
            temp_second,
            input_type,
        )?;

        let new_highest_timestamp = e_select(
            CpiContext::new(
                inco_program.clone(),
                Operation {
                    signer: self.bidder.to_account_info(),
                },
            ),
            is_gt_highest,
            enc_time_stamp,
            Euint128(self.auction.highest_timestamp),
            input_type,
        )?;

        self.auction.highest_bid = e_new_highest.0;
        self.auction.second_highest_bid = Some(new_second.0);
        self.auction.highest_timestamp = new_highest_timestamp.0;

        if remaining_accounts.len() >= 2 {
            // Allow bidder to decrypt bidder ATA balance handle
            let bidder_acc = inco_token::IncoAccount::try_deserialize(
                &mut &self.bidder_token_ata.try_borrow_data()?[..],
            )?;
            let bidder_amount_handle = bidder_acc.amount.0;

            let cpi_ctx = CpiContext::new(
                inco_program.clone(),
                Allow {
                    allowance_account: remaining_accounts[0].clone(),
                    signer: self.bidder.to_account_info(),
                    allowed_address: remaining_accounts[1].clone(),
                    system_program: self.system_program.to_account_info(),
                },
            );
            allow(cpi_ctx, bidder_amount_handle, true, self.bidder.key())?;
        }

        Ok(())
    }
}
