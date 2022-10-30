import { PublicKey, Keypair, Struct } from "@solana/web3.js";
import {
  publicKey,
  struct,
  u32,
  u64,
  u8,
  option,
  vec,
  str,
  f64,
} from "@project-serum/borsh";
import { Float } from "@solana/buffer-layout";

export enum TokenPoolInstructions {
  InitializePool = 0,
  AddMember = 1,
  SellShare = 2,
  BuyShare = 3,
  UpgradeShare = 4,
  ListNFT = 5,
  buyNft = 6,
  setManager = 7,
}

export class Payload extends Struct {
  constructor(properties: any) {
    super(properties);
  }
}

export enum AccountType {
  Uninitialized = 0,
  TokenPoolMember = 1,
}

export interface TokenPoolHeader {
  accountType: AccountType;
  maxMembers: number;
}

export enum ShareStage {
  Uninitialized = 0,
  Hold = 1,
  Escrowed = 2,
}

export interface PoolMemberShareInfo {
  accountType: AccountType;
  memberKey: PublicKey;
  amountDeposited: bigint;
  share: number;
  shareStage: ShareStage;
  escrow: PublicKey;
}

export interface PoolMemberList {
  header: TokenPoolHeader;
  members: PoolMemberShareInfo[];
}

export interface TokenPool {
  stage: number;
  targetAmount: bigint;
  minimumExemptionAmount: bigint;
  minimumAmount: bigint;
  currentBalance: bigint;
  targetToken: PublicKey;
  description: Uint8Array;
  vault: PublicKey;
  manager: PublicKey;
  treasury: PublicKey;
  poolMemberList: PoolMemberList;
}

export const HEADER_LAYOUT = [u8("accountType"), u32("maxMembers")];

export const POOL_MEMBER_SHARE_INFO_LAYOUT = struct<PoolMemberShareInfo>([
  u8("accountType"),
  publicKey("memberKey"),
  u64("amountDeposited"),
  f64("share"),
  u8("shareStage"),
  publicKey("escrow"),
]);

export const POOL_MEMBER_LIST_LAYOUT = [
  struct(HEADER_LAYOUT, "header"),
  vec(POOL_MEMBER_SHARE_INFO_LAYOUT, "members"),
];

export const TOKEN_POOL_LAYOUT = struct<TokenPool>([
  u8("stage"),
  u64("targetAmount"),
  u64("minimumExemptionAmount"),
  u64("minimumAmount"),
  u64("currentBalance"),
  publicKey("targetToken"),
  str("description"),
  publicKey("vault"),
  publicKey("manager"),
  publicKey("treasury"),
  struct(POOL_MEMBER_LIST_LAYOUT, "poolMemberList"),
]);

export enum EscrowStage {
  Uninitialized = 0,
  Initialized = 1,
  NftDeposited = 2,
  NftSold = 3,
}

export interface Escrow {
  stage: EscrowStage;
  seller: PublicKey;
  buyer: PublicKey;
  escrowVault: PublicKey;
  share: number;
  nft: PublicKey;
  nftMint: PublicKey;
  amount: bigint;
}

export const ESCROW_LAYOUT = struct<Escrow>([
  u8("stage"),
  publicKey("seller"),
  publicKey("buyer"),
  publicKey("escrowVault"),
  f64("share"),
  publicKey("nft"),
  publicKey("nftMint"),
  u64("amount"),
]);

export const getPayload = (
  instruction: u8,
  amount: bigint,
  minimumAmount: bigint,
  description: string,
  members: u32,
  minimumExemptionAmount?: bigint
) => {
  return new Payload({
    id: instruction,
    amount,
    minimumAmount,
    description,
    members,
    minimumExemptionAmount,
  });
};

export const schema = new Map([
  [
    Payload,
    {
      kind: "struct",
      fields: [
        ["id", "u8"],
        ["amount", "u64"],
        ["minimumAmount", "u64"],
        ["description", "string"],
        ["members", "u32"],
        ["minimumExemptionAmount", "u64"],
      ],
    },
  ],
]);
