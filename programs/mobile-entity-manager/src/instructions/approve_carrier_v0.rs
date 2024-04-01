use crate::state::*;
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct ApproveCarrierV0<'info> {
  pub authority: Signer<'info>,

  #[account(mut)]
  pub carrier: Box<Account<'info, CarrierV0>>,
}

pub fn handler(ctx: Context<ApproveCarrierV0>) -> Result<()> {
  ctx.accounts.carrier.approved = true;

  Ok(())
}
