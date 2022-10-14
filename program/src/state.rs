use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use solana_program::{
    msg,
    program_error::ProgramError,
    program_pack::{Pack, Sealed},
    pubkey::Pubkey,
};

#[derive(BorshDeserialize, BorshSerialize, Clone, Debug, PartialEq)]
pub struct TokenPool {
    pub target_amount: u64,               //8
    pub current_balance: u64,             //8
    pub target_token: Pubkey,             //32
    pub description: String,              //24
    pub vault: Pubkey,                    //32
    pub manager: Pubkey,                  //32
    pub treasurey: Pubkey,                //32
    pub pool_member_list: PoolMemberList, // TokenPoolHeader + PoolMemberShareInfo*max_members
}

impl TokenPool {
    // find the share percent for the amount deposited in the pool
    pub fn find_share(&self, amount: u64) -> Option<u64> {
        let share = (amount / self.target_amount) * 100;
        Some(share)
    }
}

#[derive(BorshDeserialize, BorshSerialize, Clone, Debug, PartialEq)]
pub struct PoolMemberList {
    pub header: TokenPoolHeader,           // 5
    pub members: Vec<PoolMemberShareInfo>, // (32 + 8 + 8)*max_members
}

#[derive(BorshDeserialize, BorshSerialize, Clone, Debug, PartialEq, BorshSchema)]
pub enum AccountType {
    Uninitialized = 0,
    TokenPoolMember = 1,
}

impl Default for AccountType {
    fn default() -> Self {
        AccountType::Uninitialized
    }
}

#[derive(BorshDeserialize, BorshSerialize, Clone, PartialEq, Debug, BorshSchema)]
pub struct TokenPoolHeader {
    pub account_type: AccountType, // 1 , should be TokenPoolMember
    pub max_members: u32,          // 4
}

impl TokenPoolHeader {
    const LEN: usize = 1 + 4 as usize;
}

#[derive(BorshDeserialize, BorshSerialize, Clone, PartialEq, Debug, Default, BorshSchema)]
pub struct PoolMemberShareInfo {
    pub account_type: AccountType, // 1
    pub member_key: Pubkey,        // 32
    pub amount_deposited: u64,     // 8
    pub share: u64,                // 8
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

impl PoolMemberList {
    // initializing the list with default values before assigning actual values
    pub fn new(max_members: u32) -> Self {
        Self {
            header: TokenPoolHeader {
                account_type: AccountType::TokenPoolMember,
                max_members,
            },
            members: vec![PoolMemberShareInfo::default(); max_members as usize],
        }
    }
    // get the position of the member if uninitialized else return none
    pub fn get_empty_member_index(&self) -> Option<usize> {
        self.members
            .iter()
            .position(|x| x.account_type == AccountType::Uninitialized)
    }

    // add the member in the pool with the amount deposited to pool
    pub fn add_member(
        &mut self,
        index: usize,
        member_key: Pubkey,
        amount_deposited: u64,
        share: u64,
    ) {
        self.members[index] = PoolMemberShareInfo {
            account_type: AccountType::TokenPoolMember,
            member_key,
            amount_deposited,
            share,
        }
    }

    // calculating the maximum members that can occupy the pool
    pub fn calculate_max_members(buffer_length: usize) -> usize {
        let header_size = TokenPoolHeader::LEN + 4; // adding extra 4 for metadata , need to confirm
                                                    // subtracting header size from the buffer and dividing it from PoolMemberShareInfo unit len to find the number of the members
        buffer_length.saturating_sub(header_size) / PoolMemberShareInfo::LEN
    }
}
