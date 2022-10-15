import {
    PublicKey,
    Keypair,
    Struct,
} from "@solana/web3.js";
import { publicKey, struct, u32, u64, u8, option, vec ,str, f64} from '@project-serum/borsh';
import { Float } from "@solana/buffer-layout";


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
    accountType : AccountType,
    memberKey: PublicKey,    
    amountDeposited: bigint, 
    share: number,
}

export interface PoolMemberList{
    header : TokenPoolHeader,
    members : PoolMemberShareInfo[]
}

export interface TokenPool {
    targetAmount : bigint;
    minimumAmount : bigint;
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
    u8("accountType"),
    publicKey("memberKey"),
    u64("amountDeposited"),
    f64("share")
])

export const POOL_MEMBER_LIST_LAYOUT = [
    struct(HEADER_LAYOUT,"header"),
    vec(POOL_MEMBER_SHARE_INFO_LAYOUT,"members")
]

export const TOKEN_POOL_LAYOUT = struct<TokenPool>([
    u64("targetAmount"),
    u64("minimumAmount"),
    u64("currentBalance"),
    publicKey("targetToken"),
    str("description"),
    publicKey("vault"),
    publicKey("manager"),
    publicKey("treasury"),
    struct(POOL_MEMBER_LIST_LAYOUT,"poolMemberList")
])

export const getPayload = (instruction:u8,amount:bigint,minimumAmount:bigint,description:string,members:u32) => {
    return new Payload({
        id:instruction,
        amount,
        minimumAmount,
        description,
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
          ["minimumAmount", "u64"],
          ["description", "string"],
          ["members", "u32"],
        ],
      },
    ],
]);
