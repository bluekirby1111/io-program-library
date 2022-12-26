use crate::{current_epoch, state::*, utils::*};
use anchor_lang::{prelude::*, solana_program::instruction::Instruction};
use anchor_spl::token::{Mint, TokenAccount};
use clockwork_sdk::{cpi::thread_create, state::Trigger, ThreadProgram};

use voter_stake_registry::{
  self,
  program::VoterStakeRegistry,
  state::{PositionV0, Registrar},
};

#[derive(Accounts)]
pub struct StakeV0<'info> {
  #[account(
    seeds = [b"position".as_ref(), mint.key().as_ref()],
    seeds::program = vsr_program.key(),
    bump = position.bump_seed,
    has_one = mint,
    has_one = registrar,
  )]
  pub position: Box<Account<'info, PositionV0>>,
  pub mint: Box<Account<'info, Mint>>,
  #[account(
    token::mint = mint,
    token::authority = position_authority,
    constraint = position_token_account.amount > 0
  )]
  pub position_token_account: Box<Account<'info, TokenAccount>>,
  #[account(mut)]
  pub position_authority: Signer<'info>,
  pub registrar: AccountLoader<'info, Registrar>,
  pub dao: Box<Account<'info, DaoV0>>,
  #[account(
    mut,
    has_one = dao,
  )]
  pub sub_dao: Box<Account<'info, SubDaoV0>>,

  #[account(
    init,
    space = 60 + 8 + std::mem::size_of::<StakePositionV0>(),
    payer = position_authority,
    seeds = ["stake_position".as_bytes(), position.key().as_ref()],
    bump,
  )]
  pub stake_position: Box<Account<'info, StakePositionV0>>,

  pub vsr_program: Program<'info, VoterStakeRegistry>,
  pub system_program: Program<'info, System>,
  pub clock: Sysvar<'info, Clock>,
  pub rent: Sysvar<'info, Rent>,

  /// CHECK: handled by thread_create
  #[account(
    mut,
    seeds = [b"thread", stake_position.key().as_ref(), b"purge"],
    seeds::program = clockwork.key(),
    bump
  )]
  pub thread: AccountInfo<'info>,
  pub clockwork: Program<'info, ThreadProgram>,
}

pub fn handler(ctx: Context<StakeV0>) -> Result<()> {
  // load the vehnt information
  let position = &mut ctx.accounts.position;
  let registrar = &ctx.accounts.registrar.load()?;
  let voting_mint_config = &registrar.voting_mints[position.voting_mint_config_idx as usize];
  let curr_ts = registrar.clock_unix_timestamp();
  let available_vehnt = position.voting_power(voting_mint_config, curr_ts)?;

  let seconds_left = position.lockup.seconds_left(curr_ts);
  let future_ts = curr_ts
    .checked_add(seconds_left.try_into().unwrap())
    .unwrap();
  let future_vehnt = position.voting_power(voting_mint_config, future_ts)?;

  let fall_rate = calculate_fall_rate(available_vehnt, future_vehnt, seconds_left).unwrap();

  let curr_epoch = current_epoch(curr_ts);

  let sub_dao = &mut ctx.accounts.sub_dao;
  let stake_position = &mut ctx.accounts.stake_position;

  // update the stake
  update_subdao_vehnt(sub_dao, curr_ts);
  sub_dao.vehnt_staked = sub_dao
    .vehnt_staked
    .checked_add(i128::from(available_vehnt))
    .unwrap();
  sub_dao.vehnt_fall_rate = sub_dao
    .vehnt_fall_rate
    .checked_sub(stake_position.fall_rate)
    .unwrap()
    .checked_add(fall_rate)
    .unwrap();

  if stake_position.last_claimed_epoch == 0 {
    // init stake position
    stake_position.purged = false;
    stake_position.expiry_ts = curr_ts
      .checked_add(position.lockup.seconds_left(curr_ts).try_into().unwrap())
      .unwrap();

    // init the clockwork thread to purge the position when it expires
    let signer_seeds: &[&[&[u8]]] = &[&[
      "stake_position".as_bytes(),
      position.to_account_info().key.as_ref(),
      &[ctx.bumps["stake_position"]],
    ]];

    let seconds_until_expiry = position.lockup.seconds_left(curr_ts);
    let expiry_ts = curr_ts
      .checked_add(seconds_until_expiry.try_into().unwrap())
      .unwrap();
    let cron = create_cron(expiry_ts, (60 * 60 * 2).try_into().unwrap());

    // build clockwork kickoff ix
    let accounts = vec![
      AccountMeta::new_readonly(position.key(), false),
      AccountMeta::new_readonly(ctx.accounts.dao.key(), false),
      AccountMeta::new(ctx.accounts.sub_dao.key(), false),
      AccountMeta::new(stake_position.key(), false),
      AccountMeta::new_readonly(ctx.accounts.vsr_program.key(), false),
      AccountMeta::new_readonly(ctx.accounts.clock.key(), false),
      AccountMeta::new_readonly(ctx.accounts.system_program.key(), false),
      AccountMeta::new(ctx.accounts.thread.key(), false),
      AccountMeta::new_readonly(ctx.accounts.clockwork.key(), false),
    ];
    let purge_ix = Instruction {
      program_id: crate::ID,
      accounts,
      data: clockwork_sdk::utils::anchor_sighash("purge_position_v0").to_vec(),
    };

    // initialize thread
    thread_create(
      CpiContext::new_with_signer(
        ctx.accounts.clockwork.to_account_info(),
        clockwork_sdk::cpi::ThreadCreate {
          authority: stake_position.to_account_info(),
          payer: ctx.accounts.position_authority.to_account_info(),
          thread: ctx.accounts.thread.to_account_info(),
          system_program: ctx.accounts.system_program.to_account_info(),
        },
        signer_seeds,
      ),
      "purge".to_string(),
      purge_ix.into(),
      Trigger::Cron {
        schedule: cron,
        skippable: false,
      },
    )?;
  }
  stake_position.hnt_amount = position.amount_deposited_native;
  stake_position.last_claimed_epoch = curr_epoch;
  stake_position.fall_rate = fall_rate;
  stake_position.sub_dao = ctx.accounts.sub_dao.key();
  stake_position.mint = ctx.accounts.mint.key();
  stake_position.position = ctx.accounts.position.key();
  stake_position.bump_seed = ctx.bumps["stake_position"];

  Ok(())
}
