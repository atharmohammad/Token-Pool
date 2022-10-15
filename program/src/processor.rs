use crate::id;
use crate::instructions::*;
use crate::state::*;
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::borsh::try_from_slice_unchecked;
use solana_program::program::invoke_signed;
use solana_program::system_instruction::create_account;
use solana_program::system_instruction::transfer;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    entrypoint::ProgramResult,
    msg,
    program::invoke,
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
    rent::Rent,
    sysvar::Sysvar,
};
use spl_token::instruction::{set_authority, transfer as token_transfer};

pub fn process_instruction(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    msg!("program starts!");
    let instruction = Payload::try_from_slice(input)?;
    match instruction.variant {
        0 => {
            msg!("Initialize pool instruction starts !");
            let accounts_iter = &mut accounts.iter();
            let manager_info = next_account_info(accounts_iter)?;
            let vault_info = next_account_info(accounts_iter)?;
            let target_token = next_account_info(accounts_iter)?;
            let token_pool_info = next_account_info(accounts_iter)?;
            let treasury_info = next_account_info(accounts_iter)?;
            let rent = Rent::from_account_info(next_account_info(accounts_iter)?)?;
            let _token_program = next_account_info(accounts_iter)?;

            if !rent.is_exempt(token_pool_info.lamports(), token_pool_info.data_len()) {
                return Err(ProgramError::AccountNotRentExempt);
            }

            if instruction.arg1 < instruction.arg2 {
                return Err(ProgramError::InvalidAccountData); // TO DO , need to change to custom error
            }

            msg!("Deserialize token pool account !");
            let mut token_pool =
                try_from_slice_unchecked::<TokenPool>(&token_pool_info.data.borrow())?;
            let pool_members_list: PoolMemberList = PoolMemberList::new(instruction.arg4);
            token_pool.current_balance = 0;
            token_pool.description = instruction.arg3;
            token_pool.target_amount = instruction.arg1;
            token_pool.manager = *manager_info.key;
            token_pool.target_token = *target_token.key;
            token_pool.treasurey = *treasury_info.key;
            token_pool.vault = *vault_info.key;
            token_pool.minimum_amount = instruction.arg2;
            token_pool.pool_member_list = pool_members_list;

            msg!("Serialize the data in token pool account !");
            token_pool.serialize(&mut *token_pool_info.data.borrow_mut())?;

            Ok(())
        }
        1 => {
            msg!("add member to token pool instruction starts !");
            let accounts_iter = &mut accounts.iter();
            let member_info = next_account_info(accounts_iter)?;
            let token_pool_info = next_account_info(accounts_iter)?;
            let treasury_info = next_account_info(accounts_iter)?;
            let system_program_info = next_account_info(accounts_iter)?;

            msg!("Deserialize token pool account !");
            let mut token_pool =
                try_from_slice_unchecked::<TokenPool>(&token_pool_info.data.borrow())?;

            // Check if member contributing minimum amount to be added in pool
            if instruction.arg1 < token_pool.minimum_amount {
                return Err(ProgramError::InsufficientFunds);
            }

            if token_pool.pool_member_list.find_member(*member_info.key) {
                return Err(ProgramError::InvalidAccountData); // TO DO , need to change to custom error
            }

            // TO DO ,last member should give all the left out amount to be added

            // check if the current balance is already reached the target balance
            if token_pool.current_balance == token_pool.target_amount {
                return Err(ProgramError::InsufficientFunds); // to do , need to change to custion error
            }

            let first_empty_member = token_pool.pool_member_list.get_empty_member_index();
            if first_empty_member.is_none() {
                return Err(ProgramError::InvalidArgument); // to do , need to change to custom error
            }

            /* check if the amount depositing is greater than amount left to reach target ,
            if this is the case only deposited amount needed to reach the target amount */

            let max_amount = token_pool.target_amount - token_pool.current_balance;
            let mut depositable_amount = instruction.arg1;
            if max_amount < depositable_amount {
                depositable_amount = max_amount;
            }

            msg!("add the pool member !");
            let first_empty_member = first_empty_member.unwrap();
            let share = token_pool.find_share(depositable_amount).unwrap();
            token_pool.pool_member_list.add_member(
                first_empty_member,
                *member_info.key,
                depositable_amount,
                share,
            );

            msg!("move the lamports to token pool treasury !");
            // vault is owned by the token pool account and we can credit using system account and would deduct using token pool account
            let transfer_inst = transfer(member_info.key, treasury_info.key, depositable_amount);
            invoke(
                &transfer_inst,
                &[
                    member_info.clone(),
                    treasury_info.clone(),
                    system_program_info.clone(),
                ],
            )?;

            token_pool.current_balance += depositable_amount;

            msg!("Serialize the data in token pool account !");
            token_pool.serialize(&mut *token_pool_info.data.borrow_mut())?;

            Ok(())
        }
        _ => return Err(ProgramError::InvalidArgument),
    }
}
