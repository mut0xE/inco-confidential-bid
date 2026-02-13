use anchor_lang::prelude::error_code;
#[error_code]
pub enum AuctionError {
    #[msg("Invalid start time")]
    InvalidStartTime,
    #[msg("Invalid end time")]
    InvalidEndTime,
    #[msg("Invalid reserve price")]
    InvalidReservePrice,
    #[msg("Insufficient balance")]
    InsufficientBalance,
    #[msg("Auction has not started yet")]
    AuctionNotStarted,
    #[msg("Auction has ended")]
    AuctionEnded,
    #[msg("Token amount must be greater than zero")]
    InvalidTokenAmount,
    #[msg("Inco Mint is Invalid")]
    InvalidBidMint,
    #[msg("Invalid bid amount")]
    InvalidBidAmount,
    #[msg("Auction is not open")]
    AuctionNotOpen,
    #[msg("Invalid auction")]
    InvalidAuction,
    #[msg("Invalid bid vault")]
    InvalidBidVault,
    #[msg("Math Overflow")]
    MathOverflow,
    #[msg("Bid amount is below reserve price")]
    BidBelowReserve,
    #[msg("Unauthorized")]
    Unauthorized,
    #[msg("Auction not ended")]
    AuctionNotEnded,
    #[msg("Auction must be closed")]
    AuctionNotClosed,
}
