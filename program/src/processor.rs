use crate::error::TokenPoolError;
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
use spl_token::instruction::AuthorityType;
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

            // check if target amount is less than minimum amount to be member
            if instruction.arg1 < instruction.arg2 {
                return Err(TokenPoolError::WrongAmountData.into());
            }

            // check if max members is atleast 2 to make a token pool
            if instruction.arg4 < 2 {
                return Err(TokenPoolError::MaxMemberAtleastTwo.into());
            }

            let share_sent = instruction.arg5;

            msg!("Deserialize token pool account !");
            let mut token_pool =
                try_from_slice_unchecked::<TokenPool>(&token_pool_info.data.borrow())?;

            msg!("{}", share_sent);

            let pool_members_list: PoolMemberList = PoolMemberList::new(instruction.arg4);
            token_pool.current_balance = 0;
            token_pool.minimum_exemption_amount = share_sent;
            token_pool.description = instruction.arg3;
            token_pool.target_amount = instruction.arg1;
            token_pool.manager = *manager_info.key;
            token_pool.target_token = *target_token.key;
            token_pool.treasurey = *treasury_info.key;
            token_pool.vault = *vault_info.key;
            token_pool.minimum_amount = instruction.arg2;
            token_pool.pool_member_list = pool_members_list;
            token_pool.stage = TokenPoolStage::Initialized;

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

            // check if token pool is initialized
            if token_pool.stage != TokenPoolStage::Initialized {
                return Err(TokenPoolError::UninitializedTokenPool.into());
            }

            // check if the current balance is already reached the target balance
            if token_pool.current_balance >= token_pool.target_amount {
                return Err(TokenPoolError::TargetBalanceReached.into());
            }

            let max_amount = token_pool.target_amount - token_pool.current_balance;

            // Check if member contributing minimum amount to be added in pool and amount left to reach target amount is more than minimum amount
            if instruction.arg1 < token_pool.minimum_amount
                && max_amount >= token_pool.minimum_amount
            {
                return Err(ProgramError::InsufficientFunds);
            }

            // if member already exists in the pool then he can only update his share using update share instruction
            if token_pool.pool_member_list.find_member(*member_info.key) {
                return Err(TokenPoolError::MemberAlreadyExists.into()); // TO DO , need to change to custom error
            }

            // finding the first uninitialized member
            let first_empty_member = token_pool.pool_member_list.get_empty_member_index();
            if first_empty_member.is_none() {
                return Err(TokenPoolError::NoMemberSpaceLeft.into()); // to do , need to change to custom error
            }

            // last member should give all the left out amount need to be added to reach the target amount
            if first_empty_member.unwrap()
                == (token_pool.pool_member_list.header.max_members - 1)
                    .try_into()
                    .unwrap()
            {
                if instruction.arg1 < max_amount {
                    return Err(TokenPoolError::InsufficientFundsAsLastMember.into());
                    // to do , need to change to custom error
                }
            }

            /* check if the amount depositing is greater than amount left to reach target ,
            if this is the case only deposited amount needed to reach the target amount */

            let mut depositable_amount = instruction.arg1;
            if max_amount < depositable_amount {
                depositable_amount = max_amount;
            }

            msg!("add the pool member !");
            let first_empty_member = first_empty_member.unwrap();
            let mut share = token_pool.find_share(depositable_amount).unwrap();

            token_pool.pool_member_list.add_member(
                first_empty_member,
                *member_info.key,
                depositable_amount,
                share,
            );

            msg!("move the lamports to token pool treasury !");
            // treasury is owned by the token pool account and we can credit using system account and would deduct using token pool account
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
        2 => {
            msg!("sell share instruction starts !");
            let accounts_iter = &mut accounts.iter();
            let member_info = next_account_info(accounts_iter)?;
            let token_pool_info = next_account_info(accounts_iter)?;
            let escrow_state_info = next_account_info(accounts_iter)?;
            let escrow_vault_info = next_account_info(accounts_iter)?;
            /* Create an escrow for selling share */

            msg!("Deserialize token pool account !");
            let mut token_pool =
                try_from_slice_unchecked::<TokenPool>(&token_pool_info.data.borrow())?;

            // check if token pool is initialized or not
            if token_pool.stage != TokenPoolStage::Initialized {
                return Err(TokenPoolError::UninitializedTokenPool.into());
            }

            // check if member who is selling his share is part of the pool or not
            if !token_pool.pool_member_list.find_member(*member_info.key) {
                return Err(TokenPoolError::MemberNotInPool.into());
            }

            msg!("Deserialize escrow state account !");
            let mut escrow_state = Escrow::unpack_unchecked(*escrow_state_info.data.borrow())?;

            if escrow_state.stage != EscrowStage::Uninitialized {
                return Err(TokenPoolError::InvalidEscrowStage.into());
            }
            // vault should depend on seller and token pool keys
            escrow_state.stage = EscrowStage::Initialized;
            escrow_state.amount = instruction.arg1;
            escrow_state.seller = *member_info.key;
            escrow_state.nft = token_pool.target_token;
            escrow_state.share = token_pool
                .pool_member_list
                .get_member_share(*member_info.key);
            escrow_state.escrow_vault = *escrow_vault_info.key;

            /* give authority of the share to vault and init escrow*/
            token_pool.pool_member_list.init_escrow(
                *member_info.key,
                *escrow_state_info.key,
                *escrow_vault_info.key,
            );
            token_pool.serialize(&mut *token_pool_info.data.borrow_mut())?;
            msg!("serialize escrow strate account after initializing !");
            escrow_state.serialize(&mut &mut escrow_state_info.data.borrow_mut()[..])?;

            Ok(())
        }
        3 => {
            msg!("buy share instruction starts !");
            let accounts_iter = &mut accounts.iter();
            let buyer_info = next_account_info(accounts_iter)?;
            let token_pool_info = next_account_info(accounts_iter)?;
            let escrow_state_info = next_account_info(accounts_iter)?;
            let escrow_vault_info = next_account_info(accounts_iter)?;
            let seller_info = next_account_info(accounts_iter)?;
            let system_program_info = next_account_info(accounts_iter)?;
            /* take ownership of share from the escrow */
            msg!("Deserialize token pool account !");
            let mut token_pool =
                try_from_slice_unchecked::<TokenPool>(&token_pool_info.data.borrow())?;

            // check if token pool is initialized or not
            if token_pool.stage != TokenPoolStage::Initialized {
                return Err(TokenPoolError::UninitializedTokenPool.into());
            }

            let escrow_state = Escrow::unpack_unchecked(&mut escrow_state_info.data.borrow())?;
            // check if buying amount is correct
            let buying_amount = instruction.arg1;
            if buying_amount != escrow_state.amount {
                return Err(TokenPoolError::WrongAmountData.into());
            }
            // check if escrow account has same escrow vault
            if escrow_state.escrow_vault != *escrow_vault_info.key {
                return Err(TokenPoolError::InvalidData.into());
            }
            // check if the escrow vault is member of pool list
            if !token_pool
                .pool_member_list
                .find_member(*escrow_vault_info.key)
            {
                return Err(TokenPoolError::InvalidData.into());
            }

            // check if buyer is part of the token pool then increase his share instead of adding them as member again
            let is_buyer_member = token_pool.pool_member_list.find_member(*buyer_info.key);
            if is_buyer_member {
                // upgrade buyers share
                let increased_share = token_pool
                    .pool_member_list
                    .get_member_share(*escrow_vault_info.key);
                token_pool.pool_member_list.increase_member_share(
                    increased_share,
                    *buyer_info.key,
                    buying_amount,
                );
                // remove the escrow vault from the members list
                token_pool
                    .pool_member_list
                    .remove_member(*escrow_vault_info.key);
            } else {
                // add buyer as member and only update key as amount is the needed amount contributed to reach target amount for nft
                token_pool
                    .pool_member_list
                    .update_key(*escrow_vault_info.key, *buyer_info.key);
                // removing escrow account from the members share info
                token_pool.pool_member_list.remove_escrow(*buyer_info.key);
            }

            msg!("transfer lamports to seller");
            let transfer_inst = transfer(buyer_info.key, &escrow_state.seller, escrow_state.amount);
            invoke(
                &transfer_inst,
                &[
                    buyer_info.clone(),
                    seller_info.clone(),
                    system_program_info.clone(),
                ],
            )?;
            msg!("close escrow account and tranfer lamports to seller");
            let dest_starting_lamports = seller_info.lamports();
            **seller_info.lamports.borrow_mut() = dest_starting_lamports
                .checked_add(escrow_state_info.lamports())
                .unwrap();
            **escrow_state_info.lamports.borrow_mut() = 0;

            let mut source_data = escrow_state_info.data.borrow_mut();
            source_data.fill(0);
            msg!("serialize the token pool account");
            token_pool.serialize(&mut *token_pool_info.data.borrow_mut())?;

            Ok(())
        }
        4 => {
            msg!("upgrade share instruction starts !");
            let accounts_iter = &mut accounts.iter();
            let member_info = next_account_info(accounts_iter)?;
            let token_pool_info = next_account_info(accounts_iter)?;
            let treasury_info = next_account_info(accounts_iter)?;
            let system_program_info = next_account_info(accounts_iter)?;
            let upgrading_amount = instruction.arg1;
            msg!("Deserialize token pool account !");
            let mut token_pool =
                try_from_slice_unchecked::<TokenPool>(&token_pool_info.data.borrow())?;

            // check if token pool is initialized or not
            if token_pool.stage != TokenPoolStage::Initialized {
                return Err(TokenPoolError::UninitializedTokenPool.into());
            }

            // check if the current balance is already reached the target balance
            if token_pool.current_balance >= token_pool.target_amount {
                return Err(TokenPoolError::TargetBalanceReached.into());
            }

            if !token_pool.pool_member_list.find_member(*member_info.key) {
                return Err(TokenPoolError::MemberNotInPool.into());
            }

            let member_index = token_pool
                .pool_member_list
                .get_member_index(*member_info.key)
                .unwrap();
            let total_amount = upgrading_amount
                + token_pool.pool_member_list.members[member_index].amount_deposited;
            let new_share = token_pool.find_share(total_amount).unwrap();
            token_pool.pool_member_list.update_member_share(
                new_share,
                *member_info.key,
                total_amount,
            );
            token_pool.current_balance += upgrading_amount;

            // transfer lamports to treasury
            let tranfer_lamports =
                transfer(member_info.key, &token_pool.treasurey, upgrading_amount);
            invoke(
                &tranfer_lamports,
                &[
                    member_info.clone(),
                    treasury_info.clone(),
                    system_program_info.clone(),
                ],
            )?;

            // serailize the data
            msg!("serialize the token pool account");
            token_pool.serialize(&mut *token_pool_info.data.borrow_mut())?;

            Ok(())
        }
        5 => {
            msg!("Lsit your nft instruction starts !");
            let accounts_iter = &mut accounts.iter();
            let seller_info = next_account_info(accounts_iter)?;
            let escrow_state_info = next_account_info(accounts_iter)?;
            let nft_mint_info = next_account_info(accounts_iter)?;
            let vault_info = next_account_info(accounts_iter)?;
            let nft_info = next_account_info(accounts_iter)?;
            let token_program_info = next_account_info(accounts_iter)?;
            let selling_amount = instruction.arg1;

            msg!("Deserialize escrow pool account !");
            let mut escrow = Escrow::unpack_unchecked(&escrow_state_info.data.borrow())?;

            escrow.amount = selling_amount;
            escrow.seller = *seller_info.key;
            escrow.share = 100.0;
            escrow.escrow_vault = *vault_info.key;
            escrow.stage = EscrowStage::Initialized;
            escrow.nft = *nft_info.key;
            escrow.nft_mint = *nft_mint_info.key;

            let transfer_authority = set_authority(
                token_program_info.key,
                nft_info.key,
                Some(vault_info.key),
                AuthorityType::AccountOwner,
                seller_info.key,
                &[seller_info.key],
            )?;
            invoke(
                &transfer_authority,
                &[
                    token_program_info.clone(),
                    nft_info.clone(),
                    vault_info.clone(),
                    seller_info.clone(),
                ],
            )?;

            let transfer_mint_authority = set_authority(
                token_program_info.key,
                nft_mint_info.key,
                Some(vault_info.key),
                AuthorityType::MintTokens,
                seller_info.key,
                &[seller_info.key],
            )?;
            invoke(
                &transfer_mint_authority,
                &[
                    token_program_info.clone(),
                    nft_mint_info.clone(),
                    vault_info.clone(),
                    seller_info.clone(),
                ],
            )?;

            let transfer_freeze_authority = set_authority(
                token_program_info.key,
                nft_mint_info.key,
                Some(vault_info.key),
                AuthorityType::FreezeAccount,
                seller_info.key,
                &[seller_info.key],
            )?;
            invoke(
                &transfer_freeze_authority,
                &[
                    token_program_info.clone(),
                    nft_mint_info.clone(),
                    vault_info.clone(),
                    seller_info.clone(),
                ],
            )?;

            escrow.serialize(&mut &mut escrow_state_info.data.borrow_mut()[..])?;

            Ok(())
        }
        6 => {
            msg!("Buy NFT using token pool treasury !");
            let accounts_iter = &mut accounts.iter();
            let buyer_info = next_account_info(accounts_iter)?;
            let escrow_state_info = next_account_info(accounts_iter)?;
            let token_pool_vault_info = next_account_info(accounts_iter)?;
            let nft_info = next_account_info(accounts_iter)?;
            let token_pool_info = next_account_info(accounts_iter)?;
            let treasury_info = next_account_info(accounts_iter)?;
            let seller_info = next_account_info(accounts_iter)?;
            let nft_mint_info = next_account_info(accounts_iter)?;
            let escrow_vault_info = next_account_info(accounts_iter)?;
            let manager_info = next_account_info(accounts_iter)?;
            let token_program_info = next_account_info(accounts_iter)?;

            let buying_amount = instruction.arg1;

            msg!("Deserialize token pool account !");
            let mut token_pool =
                try_from_slice_unchecked::<TokenPool>(&token_pool_info.data.borrow())?;

            // check if token pool is initialized
            if token_pool.stage != TokenPoolStage::Initialized {
                return Err(TokenPoolError::InvalidData.into());
            }

            msg!("Deserialize escrow pool account !");
            let mut escrow = Escrow::unpack_unchecked(&escrow_state_info.data.borrow())?;

            //check if buyer is part of token pool or not
            if !token_pool.pool_member_list.find_member(*buyer_info.key) {
                return Err(TokenPoolError::MemberNotInPool.into());
            }

            // check if token pool has reached the target amount
            // check if token pool's treasury have enough funds
            msg!("check if token pool's treasury have enough funds !");
            if buying_amount != escrow.amount
                && token_pool.current_balance != token_pool.target_amount
            {
                return Err(TokenPoolError::WrongAmountData.into());
            }

            msg!("check if nft is the one that was part of the token pool !");
            // check if nft is the one that was part of the token pool
            if token_pool.target_token != *nft_mint_info.key {
                return Err(TokenPoolError::InvalidData.into());
            }

            msg!("transfer the funds to seller !");
            // transfer the funds to seller
            let dest_starting_lamports = treasury_info.lamports();
            **seller_info.lamports.borrow_mut() = seller_info
                .lamports()
                .checked_add(buying_amount - token_pool.minimum_exemption_amount)
                .unwrap();
            **treasury_info.lamports.borrow_mut() =
                dest_starting_lamports.checked_sub(buying_amount).unwrap();
            **manager_info.lamports.borrow_mut() = manager_info
                .lamports()
                .checked_add(token_pool.minimum_exemption_amount)
                .unwrap();

            // transfer nft's authority
            let state_seeds = vec![b"listnft".as_ref(), escrow.nft.as_ref()];
            let (_vault_pda, _bump) = Pubkey::find_program_address(state_seeds.as_slice(), &id());

            let transfer_authority = set_authority(
                token_program_info.key,
                nft_info.key,
                Some(&token_pool.vault),
                AuthorityType::AccountOwner,
                &escrow.escrow_vault,
                &[&escrow.escrow_vault],
            )?;
            invoke_signed(
                &transfer_authority,
                &[
                    token_program_info.clone(),
                    nft_info.clone(),
                    escrow_vault_info.clone(),
                    token_pool_vault_info.clone(),
                ],
                &[&[&b"listnft"[..], escrow.nft.as_ref(), &[_bump]]],
            )?;

            msg!("transfer nft's authority !");
            let transfer_mint_authority = set_authority(
                token_program_info.key,
                nft_mint_info.key,
                Some(&token_pool.vault),
                AuthorityType::MintTokens,
                &escrow.escrow_vault,
                &[&escrow.escrow_vault],
            )?;
            invoke_signed(
                &transfer_mint_authority,
                &[
                    token_program_info.clone(),
                    nft_mint_info.clone(),
                    escrow_vault_info.clone(),
                    token_pool_vault_info.clone(),
                ],
                &[&[&b"listnft"[..], escrow.nft.as_ref(), &[_bump]]],
            )?;

            msg!("transfer freeze authority !");
            let transfer_freeze_authority = set_authority(
                token_program_info.key,
                nft_mint_info.key,
                Some(&token_pool.vault),
                AuthorityType::FreezeAccount,
                &escrow.escrow_vault,
                &[&escrow.escrow_vault],
            )?;
            invoke_signed(
                &transfer_freeze_authority,
                &[
                    token_program_info.clone(),
                    nft_mint_info.clone(),
                    escrow_vault_info.clone(),
                    token_pool_vault_info.clone(),
                ],
                &[&[&b"listnft"[..], escrow.nft.as_ref(), &[_bump]]],
            )?;

            msg!("close escrow !");
            // close escrow
            let mut source_data = escrow_state_info.data.borrow_mut();
            source_data.fill(0);

            token_pool.stage = TokenPoolStage::NFTOwned;
            token_pool.current_balance = 0;
            token_pool.serialize(&mut &mut token_pool_info.data.borrow_mut()[..])?;

            Ok(())
        }
        7 => {
            msg!("Set manager instruction starts !");
            let accounts_iter = &mut accounts.iter();
            let manger_info = next_account_info(accounts_iter)?;
            let token_pool_info = next_account_info(accounts_iter)?;
            let new_manager_info = next_account_info(accounts_iter)?;

            let mut token_pool =
                try_from_slice_unchecked::<TokenPool>(&token_pool_info.data.borrow())?;

            if token_pool.manager != *manger_info.key {
                return Err(TokenPoolError::WrongManager.into());
            }

            token_pool.manager = *new_manager_info.key;

            token_pool.serialize(&mut &mut token_pool_info.data.borrow_mut()[..])?;

            Ok(())
        }
        8 => {
            msg!("Give NFT's authority instruction starts !");
            let accounts_iter = &mut accounts.iter();
            let member_info = next_account_info(accounts_iter)?;
            let token_pool_info = next_account_info(accounts_iter)?;
            let nft_mint_info = next_account_info(accounts_iter)?;
            let nft_info = next_account_info(accounts_iter)?;
            let token_pool_vault_info = next_account_info(accounts_iter)?;
            let token_program_info = next_account_info(accounts_iter)?;

            let mut token_pool =
                try_from_slice_unchecked::<TokenPool>(&token_pool_info.data.borrow())?;

            // check if member is in token pool
            if !token_pool.pool_member_list.find_member(*member_info.key) {
                return Err(TokenPoolError::MemberNotInPool.into());
            }

            // check if nft is owned by token pool
            if token_pool.stage != TokenPoolStage::NFTOwned {
                return Err(TokenPoolError::InvalidData.into());
            }

            let member_share = token_pool
                .pool_member_list
                .get_member_share(*member_info.key);
            if member_share != 100 as f64 {
                return Err(TokenPoolError::MemberDontOwnFullShare.into());
            }

            // transfer nft's authority
            let state_seeds = vec![b"pool".as_ref(), token_pool_info.key.as_ref()];
            let (_vault_pda, _bump) = Pubkey::find_program_address(state_seeds.as_slice(), &id());

            let transfer_authority = set_authority(
                token_program_info.key,
                nft_info.key,
                Some(&member_info.key),
                AuthorityType::AccountOwner,
                &token_pool.vault,
                &[&token_pool.vault],
            )?;
            invoke_signed(
                &transfer_authority,
                &[
                    token_program_info.clone(),
                    nft_info.clone(),
                    token_pool_vault_info.clone(),
                    member_info.clone(),
                ],
                &[&[&b"pool"[..], token_pool_info.key.as_ref(), &[_bump]]],
            )?;

            msg!("transfer nft's mint authority !");
            let transfer_mint_authority = set_authority(
                token_program_info.key,
                nft_mint_info.key,
                Some(&member_info.key),
                AuthorityType::MintTokens,
                &token_pool.vault,
                &[&token_pool.vault],
            )?;
            invoke_signed(
                &transfer_mint_authority,
                &[
                    token_program_info.clone(),
                    nft_mint_info.clone(),
                    token_pool_vault_info.clone(),
                    member_info.clone(),
                ],
                &[&[&b"pool"[..], token_pool_info.key.as_ref(), &[_bump]]],
            )?;

            msg!("transfer freeze authority !");
            let transfer_freeze_authority = set_authority(
                token_program_info.key,
                nft_mint_info.key,
                Some(&member_info.key),
                AuthorityType::FreezeAccount,
                &token_pool.vault,
                &[&token_pool.vault],
            )?;
            invoke_signed(
                &transfer_freeze_authority,
                &[
                    token_program_info.clone(),
                    nft_mint_info.clone(),
                    token_pool_vault_info.clone(),
                    member_info.clone(),
                ],
                &[&[&b"pool"[..], token_pool_info.key.as_ref(), &[_bump]]],
            )?;

            // close token pool account and transfer the lamports in the members account
            let starting_lamports = token_pool_info.lamports();
            **member_info.lamports.borrow_mut() = member_info
                .lamports()
                .checked_add(starting_lamports)
                .unwrap();
            **token_pool_info.lamports.borrow_mut() = 0;
            let mut source_data = token_pool_info.data.borrow_mut();
            source_data.fill(0);

            Ok(())
        }
        _ => return Err(ProgramError::InvalidArgument),
    }
}
