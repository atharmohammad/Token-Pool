use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;
#[derive(Debug, BorshDeserialize, BorshSerialize, Clone, PartialEq)]
pub struct Payload {
    pub variant: u8,
    pub arg1: u64,
    pub arg2: String,
    pub arg3: Pubkey,
    pub arg4: u32,
}
// #[derive(Debug, BorshDeserialize, BorshSerialize, Clone, PartialEq)]
pub enum TokenPoolInstructions {
    /// Initialize a token pool with a target amount for purchasing of specific token
    /// accounts required :
    /// 0 - [signer] token pool manager , who is initializing the token pool
    /// 1 - [] vault , pda which will own the token bought using the pool money
    /// 2 - [] target token , token which will be bought using pool money
    /// 3 - [writer] token pool state account
    /// 4 - [] treasury,which will store all lamports of the pool
    /// 5 - [] rent sysvar
    /// 6 - [] token program
    InitializePool {
        target_amount: u64,
        target_token: Pubkey,
        description: String,
        max_members: u32,
    },
    /// AddMember instruction adds a member and their contribution to token pool
    /// accounts required :
    /// 0 - [signer] member, who will be added to pool
    /// 1 - [writer] token pool state account
    AddMember { amount: u64 },
}
