use anchor_lang::prelude::*;

use crate::state::AuctionType;

#[event]
pub struct AuctionCreated {
    pub auction_id: u64,
    pub organizer: Pubkey,
    pub mint: Pubkey,
    pub amount: u64,
    pub decimals: u8,
    pub start_time: i64,
    pub end_time: i64,
    pub reserve_price: u64,
    pub auction_type: AuctionType,
    pub bid_token_mint: Pubkey,
}
