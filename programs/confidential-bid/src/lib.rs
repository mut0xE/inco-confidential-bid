use anchor_lang::prelude::*;

declare_id!("Ek9MekGDy6g1CAwoE8AbSXkhBXnkJDMQxTLHFKLpTFii");
mod constants;
mod error;
mod events;
mod instructions;
mod state;
use crate::state::AuctionType;
use instructions::*;
#[program]
pub mod confidential_bid {

    use super::*;

    pub fn create_auction(
        ctx: Context<CreateAuction>,
        auction_id: u64,
        start_time: i64,
        end_time: i64,
        reserve_price: u64,
        auction_type: AuctionType,
        token_amount: u64,
    ) -> Result<()> {
        ctx.accounts.handler(
            auction_id,
            start_time,
            end_time,
            reserve_price,
            auction_type,
            token_amount,
            &ctx.bumps,
        )?;
        Ok(())
    }

    pub fn place_bid<'info>(
        ctx: Context<'_, '_, '_, 'info, PlaceBid<'info>>,
        token_amount: Vec<u8>,
        input_type: u8,
    ) -> Result<()> {
        ctx.accounts.handler(
            token_amount,
            &ctx.bumps,
            input_type,
            &ctx.remaining_accounts,
        )?;
        Ok(())
    }

    pub fn close_auction(ctx: Context<CloseAuction>) -> Result<()> {
        ctx.accounts.handler()?;
        Ok(())
    }
}
