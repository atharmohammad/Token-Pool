use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;
#[derive(Debug, BorshDeserialize, BorshSerialize, Clone, PartialEq)]
pub struct Payload {
    pub variant: u8,
    pub arg1: u64,
    pub arg2: u64,
    pub arg3: String,
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
        minimum_amount: u64,
        description: String,
        max_members: u32,
    },
    /// AddMember instruction adds a member and their contribution to token pool
    /// accounts required :
    /// 0 - [signer] member, who will be added to pool
    /// 1 - [writer] token pool state account
    /// 2 - [writer] treasury , which will store all lamports of the pool
    /// 3 - [] system program
    AddMember { amount: u64 },
    /// SellShare instruction starts escrow to sell the share of a member to some other person
    /// accounts required :
    /// 0 - [signer] member, who is selling his share
    /// 1 - [writer] token pool state account
    /// 2 - [writer] escrow state account
    /// 3 - [] escrow vault
    SellShare { amount: u64 },
    /// BuyShare instruction buys the share of a nft through escrow process and compeletes escrow transacton
    /// accounts required :
    /// 0 - [signer] member, who is buying the share
    /// 1 - [writer] token pool state account
    /// 2 - [writer] escrow state account
    /// 3 - [] escrow vault
    /// 4 - [] system program
    BuyShare { amount: u64 },
    /* Instructions need to be implemented
       - Execute nft share buying
       - sell your nft share
       - upgrade your share
       - withdraw sol
       - close token pool if nft sold
    */
}
