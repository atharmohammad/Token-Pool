import {
    LAMPORTS_PER_SOL,
    sendAndConfirmTransaction,
    PublicKey,
    Connection,
    Keypair,
    Transaction,
    TransactionInstruction,
    SYSVAR_RENT_PUBKEY,
    SystemProgram,
    Struct,
    SYSVAR_CLOCK_PUBKEY,
    AccountInfo
} from "@solana/web3.js";
import {TOKEN_PROGRAM_ID,ASSOCIATED_TOKEN_PROGRAM_ID, getAssociatedTokenAddress, createAssociatedTokenAccount, AccountLayout, transfer, mintTo, createAssociatedTokenAccountInstruction, createMint, createInitializeAccountInstruction, createInitializeAccount3Instruction, createMintToInstruction, getOrCreateAssociatedTokenAccount, initializeAccountInstructionData, createSetAuthorityInstruction, AuthorityType} from "@solana/spl-token";
import fs from 'mz/fs';
import os from 'os';
import path from 'path';
import yaml from 'yaml';
import {struct,u32,u8} from "@solana/buffer-layout";
import {bigInt, publicKey, u64} from "@solana/buffer-layout-utils"
import { serialize, deserialize, deserializeUnchecked } from "borsh";
import assert from "assert";

class Payload extends Struct {
    constructor(properties : any) {
      super(properties);
    }
  }
  
// Path to local Solana CLI config file.
const CONFIG_FILE_PATH = path.resolve(
    os.homedir(),
    '.config',
    'solana',
    'cli',
    'config.yml',
);
const PROGRAM_KEYPAIR_PATH = path.join(
    path.resolve(__dirname,"../dist/program/"),"token_pool-keypair.json"
);

const createAccount = async(connection:Connection) : Promise<Keypair> => {
    const key = Keypair.generate();
    const airdrop = await connection.requestAirdrop(key.publicKey,2*LAMPORTS_PER_SOL);
    await connection.confirmTransaction(airdrop)
    return key;
}

const createKeypairFromFile = async(path:string): Promise<Keypair> => {
    const secret_keypair = await fs.readFile(path,{encoding:"utf8"});
    const secret_key = Uint8Array.from(JSON.parse(secret_keypair));
    const programKeypair = Keypair.fromSecretKey(secret_key);
    return programKeypair;
}

let value = new Payload({
    id:0,
    amount: BigInt(2345),
    description : "Monke NFT",
    target : Keypair.generate().publicKey.toBuffer(),
    members : 4
});

let schema = new Map([
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

/* Since we are saying we would have maximum of 4 members in this token pool so we would initialize the space for max of 4 members */

let connection : Connection , programId : Keypair , manager : Keypair , token_pool : Keypair , token_members_list : Keypair , vault : PublicKey;
let _vault_bump: number , target_token : Keypair , treasury : Keypair
const main = async()=>{
    const localenet = "http://127.0.0.1:8899";
    connection = new Connection(localenet);    
    programId = await createKeypairFromFile(PROGRAM_KEYPAIR_PATH);
    console.log("Pinging ... !");
    manager = await createAccount(connection);
    token_pool = Keypair.generate();
    token_members_list = Keypair.generate();
    const token_members_list_inst = SystemProgram.createAccount({
        space: (1+4) + (32+8+8)*4, //size of one PoolMemberShare * max_members
        lamports: await connection.getMinimumBalanceForRentExemption(
            (1+4) + (32+8+8)*4
        ),
        fromPubkey: manager.publicKey,
        newAccountPubkey: token_members_list.publicKey,
        programId: programId.publicKey,
    });
    const token_pool_account_inst = SystemProgram.createAccount({
        space: 8+8+32+24+32+32+32+(1+4) + (32+8+8)*4,
        lamports: await connection.getMinimumBalanceForRentExemption(
            8+8+32+24+32+32+32+(1+4) + (32+8+8)*4
        ),
        fromPubkey: manager.publicKey,
        newAccountPubkey: token_pool.publicKey,
        programId: programId.publicKey,
    });
    target_token = Keypair.generate();
    treasury = Keypair.generate();
    [vault,_vault_bump] = await PublicKey.findProgramAddress([Buffer.from("pool"),token_pool.publicKey.toBuffer()],programId.publicKey);
    const transaction_inst = new TransactionInstruction({
        keys:[
            {pubkey:manager.publicKey,isSigner:true,isWritable:true},
            {pubkey:vault,isSigner:false,isWritable:false},
            {pubkey:target_token.publicKey,isSigner:false,isWritable:false},
            {pubkey:token_pool.publicKey,isSigner:false,isWritable:true},
            {pubkey:treasury.publicKey,isSigner:false,isWritable:false},
            {pubkey:SYSVAR_RENT_PUBKEY,isSigner:false,isWritable:false},
            {pubkey:TOKEN_PROGRAM_ID,isSigner:false,isWritable:false},
        ],
        programId:programId.publicKey,
        data : Buffer.from(serialize(schema,value))
    });
    const tx = new Transaction();
    tx.add(token_members_list_inst,token_pool_account_inst,transaction_inst);
    await sendAndConfirmTransaction(connection,tx,[manager,token_members_list,token_pool]);

    const token_pool_after_buff = await connection.getAccountInfo(token_pool.publicKey);
    if(!token_pool_after_buff){
        console.log("Error!!");
        return;
    }
    console.log(token_pool_after_buff);
}

main().then(
    ()=>process.exit(),
    err =>{
        console.log(err);
        process.exit(-1);
    }
)