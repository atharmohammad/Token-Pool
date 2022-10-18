use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use solana_program::{
    msg,
    program_error::ProgramError,
    program_memory::sol_memcmp,
    program_pack::{Pack, Sealed},
    pubkey::{Pubkey, PUBKEY_BYTES},
};
use std::{borrow::{Borrow, BorrowMut}, convert::TryFrom, fmt, matches};

#[derive(BorshDeserialize, BorshSerialize, Clone, Debug, PartialEq)]
pub struct TokenPool {
    pub stage: TokenPoolStage,            //1
    pub target_amount: u64,               //8
    pub minimum_amount: u64,              //8
    pub current_balance: u64,             //8
    pub target_token: Pubkey,             //32
    pub description: String,              //24
    pub vault: Pubkey,                    //32
    pub manager: Pubkey,                  //32
    pub treasurey: Pubkey,                //32
    pub pool_member_list: PoolMemberList, // TokenPoolHeader + PoolMemberShareInfo*max_members
}

#[derive(BorshDeserialize, BorshSerialize, Clone, Debug, PartialEq)]
pub enum TokenPoolStage {
    Uninitialized = 0,
    Initialized = 1,
}

impl TokenPool {
    /// find the share percent for the amount deposited in the pool
    pub fn find_share(&self, amount: u64) -> Option<f64> {
        let share = (amount as f64 / self.target_amount as f64) * 100 as f64;
        Some(share)
    }
}

#[derive(BorshDeserialize, BorshSerialize, Clone, Debug, PartialEq)]
pub struct PoolMemberList {
    pub header: TokenPoolHeader,           // 5
    pub members: Vec<PoolMemberShareInfo>, // (1 + 32 + 8 + 8)*max_members
}

#[derive(BorshDeserialize, BorshSerialize, Copy, Clone, Debug, PartialEq, BorshSchema)]
pub enum AccountType {
    Uninitialized = 0,
    TokenPoolMember = 1,
}

impl Default for AccountType {
    fn default() -> Self {
        AccountType::Uninitialized
    }
}

#[derive(BorshDeserialize, BorshSerialize, Copy, Clone, PartialEq, Debug, BorshSchema)]
pub struct TokenPoolHeader {
    pub account_type: AccountType, // 1 , should be TokenPoolMember
    pub max_members: u32,          // 4
}

impl TokenPoolHeader {
    const LEN: usize = 1 + 4 as usize;

    pub fn deserialize_vec(data: &mut [u8]) -> Result<&mut [u8], ProgramError> {
        let len = 1 + 8 + 8 + 8 + 32 + 24 + 32 + 32 + 32 + 1 + 4;
        Ok(&mut data[len..])
    }
}

#[derive(BorshDeserialize, BorshSerialize, Clone, Copy, PartialEq, Debug, Default, BorshSchema)]
pub struct PoolMemberShareInfo {
    pub account_type: AccountType, // 1
    pub member_key: Pubkey,        // 32
    pub amount_deposited: u64,     // 8
    pub share: f64,                // 8
}

impl Sealed for PoolMemberShareInfo {}

impl Pack for PoolMemberShareInfo {
    const LEN: usize = 1 + 32 + 8 + 8;

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let mut slice = dst;
        self.serialize(&mut slice).unwrap()
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let mut p = src;
        PoolMemberShareInfo::deserialize(&mut p).map_err(|_| {
            msg!("Failed to deserialize");
            ProgramError::InvalidAccountData
        })
    }
}

impl PoolMemberShareInfo {
    /// Performs a very cheap comparison, for checking if this member share
    /// info matches the member account address
    pub fn memcmp_pubkey(data: &[u8], member_address_bytes: &[u8]) -> bool {
        sol_memcmp(
            &data[1..1+PUBKEY_BYTES],
            member_address_bytes,
            PUBKEY_BYTES,
        ) == 0
    }
}

impl PoolMemberList {
    /// initializing the list with default values before assigning actual values
    pub fn new(max_members: u32) -> Self {
        Self {
            header: TokenPoolHeader {
                account_type: AccountType::TokenPoolMember,
                max_members,
            },
            members: vec![PoolMemberShareInfo::default(); max_members as usize],
        }
    }

    pub fn update_key(&mut self,member_key: Pubkey,new_key:Pubkey) {
        let index = &self.members.iter().position(|x| x.member_key == member_key).unwrap();
        self.members[*index].member_key = new_key;
    }

    /// get the share of member in the token pool
    pub fn get_member_share(&mut self, member_key: Pubkey) -> f64 {
        let index = self
            .members
            .iter()
            .position(|x| x.member_key == member_key)
            .unwrap();
        self.members[index].share
    }

    /// find if member exists in a pool member list
    pub fn find_member(&self, member_key: Pubkey) -> bool {
        self.members.iter().any(|x| x.member_key == member_key)
    }

    /// get the first position of the member which is uninitialized else return none
    pub fn get_empty_member_index(&self) -> Option<usize> {
        self.members
            .iter()
            .position(|x| x.account_type == AccountType::Uninitialized)
    }

    /// add the member in the pool with the amount deposited to pool
    pub fn add_member(
        &mut self,
        index: usize,
        member_key: Pubkey,
        amount_deposited: u64,
        share: f64,
    ) {
        self.members[index] = PoolMemberShareInfo {
            account_type: AccountType::TokenPoolMember,
            member_key,
            amount_deposited,
            share,
        }
    }

    /// calculating the maximum members that can occupy the pool
    pub fn calculate_max_members(buffer_length: usize) -> usize {
        let header_size = TokenPoolHeader::LEN + 4; // adding extra 4 for metadata , need to confirm
                                                    // subtracting header size from the buffer and dividing it from PoolMemberShareInfo unit len to find the number of the members
        buffer_length.saturating_sub(header_size) / PoolMemberShareInfo::LEN
    }
}

#[derive(BorshDeserialize, BorshSerialize, Clone, Debug, PartialEq)]
pub enum EscrowStage {
    Uninitialized = 0,
    Initialized = 1,
    NftDeposited = 2,
    NftSold = 3,
}

impl Default for EscrowStage {
    fn default() -> Self {
        EscrowStage::Uninitialized
    }
}

// TO DO : Implement interface on client side
#[derive(BorshDeserialize, BorshSerialize, Clone, Debug, PartialEq)]
pub struct Escrow {
    pub stage: EscrowStage,   //1
    pub seller: Pubkey,       //32
    pub buyer: Pubkey,        //32
    pub escrow_vault: Pubkey, //32
    pub share: f64,           //32
    pub nft: Pubkey,          //32
    pub amount: u64,          //8
}

impl Sealed for Escrow {}

impl Pack for Escrow {
    const LEN: usize = 1 + 32 + 32 + 32 + 32 + 32 + 8;

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let mut slice = dst;
        self.serialize(&mut slice).unwrap()
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let mut p = src;
        Escrow::deserialize(&mut p).map_err(|_| {
            msg!("Failed to deserialize");
            ProgramError::InvalidAccountData
        })
    }
}
