use crate::state::*;
use anchor_lang::prelude::*;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default)]
pub struct RevokeProgramArgsV0 {
  pub program_id: Pubkey,
}

#[derive(Accounts)]
#[instruction(args: RevokeProgramArgsV0)]
pub struct RevokeProgramV0<'info> {
  #[account(mut)]
  pub refund: Signer<'info>,

  pub authority: Signer<'info>,

  #[account(
    mut,
    close = refund,
    seeds = ["program_approval".as_bytes(), args.program_id.as_ref()],
    bump = program_approval.bump_seed,
  )]
  pub program_approval: Box<Account<'info, ProgramApprovalV0>>,
  pub system_program: Program<'info, System>,
}

pub fn handler(_ctx: Context<RevokeProgramV0>, _args: RevokeProgramArgsV0) -> Result<()> {
  Ok(())
}
