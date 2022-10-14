import {
    PublicKey,
    Keypair,
    Struct,
} from "@solana/web3.js";
import { publicKey, struct, u32, u64, u8, option, vec ,str} from '@project-serum/borsh';


export class Payload extends Struct {
    constructor(properties : any) {
      super(properties);
    }
  }
  

export enum AccountType {
    Uninitialized = 0,
    TokenPoolMember = 1,
}

export interface TokenPoolHeader {
    accountType : AccountType,
    maxMembers : number
}

export interface PoolMemberShareInfo{
    memberKey: PublicKey,    
    amountDeposited: bigint, 
    share: bigint,
}

export interface PoolMemberList{
    header : TokenPoolHeader,
    members : PoolMemberShareInfo[]
}

export interface TokenPool {
    targetAmount : bigint;
    currentBalance : bigint;
    targetToken : PublicKey;
    description : Uint8Array;
    vault : PublicKey;
    manager : PublicKey;
    treasury : PublicKey;
    poolMemberList : PoolMemberList 
}

export const HEADER_LAYOUT = [
    u8("accountType"),
    u32("maxMembers")
]

export const POOL_MEMBER_SHARE_INFO_LAYOUT = struct<PoolMemberShareInfo>([
    publicKey("memberKey"),
    u64("amountDeposited"),
    u64("share")
])

export const POOL_MEMBER_LIST_LAYOUT = [
    struct(HEADER_LAYOUT,"header"),
    vec(POOL_MEMBER_SHARE_INFO_LAYOUT,"members")
]

export const TOKEN_POOL_LAYOUT = struct<TokenPool>([
    u64("targetAmount"),
    u64("currentBalance"),
    publicKey("targetToken"),
    str("description"),
    publicKey("vault"),
    publicKey("manager"),
    publicKey("treasury"),
    struct(POOL_MEMBER_LIST_LAYOUT,"poolMemberList")
])

export const getPayload = (instruction:u8,amount:bigint,description:string,target:Buffer,members:u32) => {
    return new Payload({
        id:instruction,
        amount,
        description,
        target,
        members
    });
}

export const schema = new Map([
    [
        Payload,
      {
        kind: "struct",
        fields: [
          ["id" , "u8"],
          ["amount", "u64"],
          ["description", "string"],
          ["target", [32]],
          ["members", "u32"],
        ],
      },
    ],
]);
