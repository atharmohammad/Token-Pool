use borsh::{BorshDeserialize, BorshSerialize};
#[derive(Debug, BorshDeserialize, BorshSerialize, Clone, PartialEq)]
pub struct Payload {
    pub variant: u8,
    pub arg1: u64,
    pub arg2: u64,
}
// #[derive(Debug, BorshDeserialize, BorshSerialize, Clone, PartialEq)]
pub enum TokenPoolInstructions {
   
}