use crate::id;
use crate::instructions::*;
use crate::state::*;
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::borsh::try_from_slice_unchecked;
use solana_program::program::invoke_signed;
use solana_program::system_instruction::create_account;
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
use spl_token::instruction::{set_authority, transfer};

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
            msg!("Deserialize token pool account !");
            let mut token_pool =
                try_from_slice_unchecked::<TokenPool>(&token_pool_info.data.borrow())?;
            let pool_members_list: PoolMemberList = PoolMemberList::new(instruction.arg4);
            token_pool.current_balance = 0;
            token_pool.description = instruction.arg2;
            token_pool.target_amount = instruction.arg1;
            token_pool.manager = *manager_info.key;
            token_pool.target_token = *target_token.key;
            token_pool.treasurey = *treasury_info.key;
            token_pool.vault = *vault_info.key;
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
            msg!("Deserialize token pool account !");
            let mut token_pool =
                try_from_slice_unchecked::<TokenPool>(&token_pool_info.data.borrow())?;
            let first_empty_member = token_pool.pool_member_list.get_empty_member_index();
            if first_empty_member.is_none() {
                return Err(ProgramError::InvalidArgument); // to do , need to change to custom error
            }

            msg!("add the pool member !");
            let first_empty_member = first_empty_member.unwrap();
            let share = token_pool.find_share(instruction.arg1).unwrap();
            token_pool.pool_member_list.add_member(
                first_empty_member,
                *member_info.key,
                instruction.arg1,
                share,
            );

            msg!("Serialize the data in token pool account !");
            token_pool.serialize(&mut *token_pool_info.data.borrow_mut())?;
            /* need to check if the amount depositing in greater than amount left to reach target , if this is the case only deposited amount needed to reach the target amount */
            /* move the lamports to token pool treasury */
            Ok(())
        }
        _ => return Err(ProgramError::InvalidArgument),
    }
}
