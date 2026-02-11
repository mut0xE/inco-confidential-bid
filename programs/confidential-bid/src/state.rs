use anchor_lang::prelude::*;

#[account]
pub struct AuctionState {
    pub organizer: Pubkey,
    pub mint: Pubkey, // Token being auctioned
    pub highest_bidder: Option<Pubkey>,
    pub vault: Pubkey,
    pub bid_token_mint: Pubkey, // Token used for bidding
    pub bid_vault: Pubkey,      // Inco bid  vault
    pub highest_bid: u128,
    pub second_highest_bid: Option<u128>,
    pub highest_timestamp: u128, // Encrypted timestamp of earliest highest bid
    pub start_time: i64,
    pub end_time: i64,
    pub reserve_price: u64, // Minimum bid in bid_token_mint
    pub auction_id: u64,
    pub bid_count: u32,
    pub auction_status: AuctionStatus,
    pub auction_type: AuctionType,
    pub auction_bump: u8,
}
impl AuctionState {
    pub const LEN: usize =
        32 + 32 + 1 + 32 + 32 + 32 + 16 + 1 + 16 + 16 + 8 + 8 + 8 + 8 + 4 + 1 + 1 + 1;
}

#[account]
#[derive(InitSpace)]
pub struct Bid {
    pub bidder: Pubkey,
    pub auction: Pubkey,
    pub bid_amount: u128,
    pub time_stamp: u128,
    pub bid_bump: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq)]
pub enum AuctionType {
    Normal,  // First-price: winner pays their own bid
    Vickrey, // Second-price: winner pays second-highest bid
}
#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq)]
pub enum AuctionStatus {
    Open,
    Closed,
    Settled,
    Cancelled,
}
