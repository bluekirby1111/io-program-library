use anchor_lang::prelude::*;
use anchor_spl::token::Mint;

use crate::BoostConfigV0;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default)]
pub struct InitializeBoostConfigArgsV0 {
  /// The price in the oracle (usd) to burn boost
  pub boost_price: u64,
  /// The length of a period (defined as a month in the HIP)
  pub period_length: u32,
  /// The minimum of periods to boost
  pub minimum_periods: u16,
}

#[derive(Accounts)]
#[instruction(args: InitializeBoostConfigArgsV0)]
pub struct InitializeBoostConfigV0<'info> {
  #[account(mut)]
  pub payer: Signer<'info>,

  pub authority: Signer<'info>,
  /// CHECK: Just for settings
  pub rent_reclaim_authority: AccountInfo<'info>,
  /// CHECK: Just for settings
  pub start_authority: AccountInfo<'info>,
  /// CHECK: Pyth price oracle
  pub price_oracle: AccountInfo<'info>,
  pub dnt_mint: Box<Account<'info, Mint>>,

  #[account(
    init,
    payer = payer,
    space = 8 + 60 + std::mem::size_of::<BoostConfigV0>(),
    seeds = ["boost_config".as_bytes(), dnt_mint.key().as_ref()],
    bump,
  )]
  pub boost_config: Box<Account<'info, BoostConfigV0>>,
  pub system_program: Program<'info, System>,
}

pub fn handler(
  ctx: Context<InitializeBoostConfigV0>,
  args: InitializeBoostConfigArgsV0,
) -> Result<()> {
  require_gt!(args.period_length, 0);

  ctx.accounts.boost_config.set_inner(BoostConfigV0 {
    price_oracle: ctx.accounts.price_oracle.key(),
    payment_mint: ctx.accounts.dnt_mint.key(),
    boost_price: args.boost_price,
    period_length: args.period_length,
    minimum_periods: args.minimum_periods,
    rent_reclaim_authority: ctx.accounts.rent_reclaim_authority.key(),
    bump_seed: ctx.bumps["boost_config"],
    start_authority: ctx.accounts.start_authority.key(),
  });

  Ok(())
}
