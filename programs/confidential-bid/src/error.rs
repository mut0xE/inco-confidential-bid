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
}
