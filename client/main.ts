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
  AccountInfo,
} from "@solana/web3.js";
import {
  TOKEN_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
  getAssociatedTokenAddress,
  createAssociatedTokenAccount,
  AccountLayout,
  transfer,
  mintTo,
  createAssociatedTokenAccountInstruction,
  createMint,
  createInitializeAccountInstruction,
  createInitializeAccount3Instruction,
  createMintToInstruction,
  getOrCreateAssociatedTokenAccount,
  initializeAccountInstructionData,
  createSetAuthorityInstruction,
  AuthorityType,
} from "@solana/spl-token";
import fs from "mz/fs";
import os from "os";
import path from "path";
import { serialize } from "borsh";
import assert from "assert";
import {
  AccountType,
  Escrow,
  EscrowStage,
  ESCROW_LAYOUT,
  getPayload,
  schema,
  ShareStage,
  TokenPool,
  TOKEN_POOL_LAYOUT,
} from "./layout";

// Path to local Solana CLI config file.
const CONFIG_FILE_PATH = path.resolve(
  os.homedir(),
  ".config",
  "solana",
  "cli",
  "config.yml"
);
const PROGRAM_KEYPAIR_PATH = path.join(
  path.resolve(__dirname, "../dist/program/"),
  "token_pool-keypair.json"
);

const createAccount = async (connection: Connection): Promise<Keypair> => {
  const key = Keypair.generate();
  const airdrop = await connection.requestAirdrop(
    key.publicKey,
    2 * LAMPORTS_PER_SOL
  );
  await connection.confirmTransaction(airdrop);
  return key;
};

const airdrop_sol = async (connection: Connection, key: PublicKey) => {
  const airdrop = await connection.requestAirdrop(key, 2 * LAMPORTS_PER_SOL);
  await connection.confirmTransaction(airdrop);
};

const createKeypairFromFile = async (path: string): Promise<Keypair> => {
  const secret_keypair = await fs.readFile(path, { encoding: "utf8" });
  const secret_key = Uint8Array.from(JSON.parse(secret_keypair));
  const programKeypair = Keypair.fromSecretKey(secret_key);
  return programKeypair;
};

const get_account_data = async (
  token_pool: PublicKey
): Promise<AccountInfo<Buffer>> => {
  const token_pool_after_buff = await connection.getAccountInfo(token_pool);
  if (!token_pool_after_buff) {
    console.log("Error!!");
    process.exit(-1);
  }
  return token_pool_after_buff;
};

const max_members = 4;
export const description = "Monke NFT";
const TOKEN_POOL_SIZE =
  1 +
  8 +
  8 +
  8 +
  32 +
  24 +
  32 +
  32 +
  32 +
  (1 + 4) +
  (1 + 32 + 8 + 8 + 1 + 32) * max_members;
const ESCROW_STATE_SIZE = 1 + 32 + 32 + 32 + 32 + 32 + 8;
const TOKEN_MEMBER_LIST_SIZE = 1 + 4 + (1 + 32 + 8 + 8 + 1 + 32) * max_members;

/* Since we are saying we would have maximum of 4 members in this token pool so we would initialize the space for max of 4 members */

let connection: Connection,
  programId: Keypair,
  manager: Keypair,
  token_pool: Keypair,
  token_members_list: Keypair,
  vault: PublicKey;
let _vault_bump: number, target_token: Keypair, treasury: Keypair;
const main = async () => {
  const localenet = "http://127.0.0.1:8899";
  connection = new Connection(localenet);
  programId = await createKeypairFromFile(PROGRAM_KEYPAIR_PATH);
  console.log("Pinging ... !");
  await initialize();
  await airdrop_sol(connection, manager.publicKey);
  await addMember(manager);
  await startSellEscrow(manager);
};
/*** Amount are in lamports ***/

const startSellEscrow = async (member: Keypair) => {
  const value = getPayload(2, BigInt(1), BigInt(1), description, max_members); // only id needed , all other are placeholders
  const escrow_state = Keypair.generate();
  const create_escrow_inst = SystemProgram.createAccount({
    space: ESCROW_STATE_SIZE,
    lamports: await connection.getMinimumBalanceForRentExemption(
      ESCROW_STATE_SIZE
    ),
    fromPubkey: member.publicKey,
    newAccountPubkey: escrow_state.publicKey,
    programId: programId.publicKey,
  });
  const [escrow_vault, _escrow_vault_bump] = await PublicKey.findProgramAddress(
    [
      Buffer.from("escrow"),
      member.publicKey.toBuffer(),
      token_pool.publicKey.toBuffer(),
    ],
    programId.publicKey
  );
  const transaction_inst = new TransactionInstruction({
    keys: [
      { pubkey: member.publicKey, isSigner: true, isWritable: true },
      { pubkey: token_pool.publicKey, isSigner: false, isWritable: true },
      { pubkey: escrow_state.publicKey, isSigner: false, isWritable: true },
      { pubkey: escrow_vault, isSigner: false, isWritable: false },
    ],
    programId: programId.publicKey,
    data: Buffer.from(serialize(schema, value)),
  });
  const tx = new Transaction();
  tx.add(create_escrow_inst, transaction_inst);
  await sendAndConfirmTransaction(connection, tx, [member, escrow_state]);
  const token_pool_data: Buffer = (await get_account_data(token_pool.publicKey))
    .data;
  const pool_data: TokenPool = TOKEN_POOL_LAYOUT.decode(token_pool_data);
  pool_data.poolMemberList.members[0].memberKey.equals(escrow_vault);
  const escrow_data_buffer: Buffer = (
    await get_account_data(escrow_state.publicKey)
  ).data;
  const escrow_data: Escrow = ESCROW_LAYOUT.decode(escrow_data_buffer);
  assert.equal(escrow_data.stage, EscrowStage.Initialized);
  escrow_data.seller.equals(member.publicKey);
  escrow_data.escrowVault.equals(escrow_vault);
  escrow_data.nft.equals(pool_data.targetToken);
  assert.equal(escrow_data.share, pool_data.poolMemberList.members[0].share);
  assert.equal(escrow_data.amount, 1); // amount want for the share is 1
  pool_data.poolMemberList.members[0].escrow.equals(escrow_state.publicKey);
  assert.equal(
    pool_data.poolMemberList.members[0].shareStage,
    ShareStage.Escrowed
  );
};

const addMember = async (member: Keypair) => {
  const value = getPayload(1, BigInt(1), BigInt(1), description, max_members);
  const transaction_inst = new TransactionInstruction({
    keys: [
      { pubkey: member.publicKey, isSigner: true, isWritable: true },
      { pubkey: token_pool.publicKey, isSigner: false, isWritable: true },
      { pubkey: treasury.publicKey, isSigner: false, isWritable: true },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    programId: programId.publicKey,
    data: Buffer.from(serialize(schema, value)),
  });
  const tx = new Transaction();
  tx.add(transaction_inst);
  await sendAndConfirmTransaction(connection, tx, [member]);

  const token_pool_data: Buffer = (await get_account_data(token_pool.publicKey))
    .data;
  const pool_data: TokenPool = TOKEN_POOL_LAYOUT.decode(token_pool_data);
  assert.equal(pool_data.currentBalance.toString(), "1");
  assert.equal(
    pool_data.poolMemberList.members[0].amountDeposited.toString(),
    "1"
  );
  assert.equal(
    pool_data.poolMemberList.members[0].accountType,
    AccountType.TokenPoolMember
  );
  assert.equal(pool_data.poolMemberList.members[0].share, 20);
  member.publicKey.equals(pool_data.poolMemberList.members[0].memberKey);
  const treasury_data_buffer = await get_account_data(treasury.publicKey);
  assert.equal(
    treasury_data_buffer.lamports,
    (await connection.getMinimumBalanceForRentExemption(0)) + 1
  );
};

const initialize = async () => {
  target_token = Keypair.generate();
  const value = getPayload(0, BigInt(5), BigInt(1), description, max_members);
  manager = await createAccount(connection);
  token_pool = Keypair.generate();
  token_members_list = Keypair.generate();
  const token_members_list_inst = SystemProgram.createAccount({
    space: TOKEN_MEMBER_LIST_SIZE, //size of one PoolMemberShare * max_members
    lamports: await connection.getMinimumBalanceForRentExemption(
      TOKEN_MEMBER_LIST_SIZE
    ),
    fromPubkey: manager.publicKey,
    newAccountPubkey: token_members_list.publicKey,
    programId: programId.publicKey,
  });
  const token_pool_account_inst = SystemProgram.createAccount({
    space: TOKEN_POOL_SIZE,
    lamports: await connection.getMinimumBalanceForRentExemption(
      TOKEN_POOL_SIZE
    ),
    fromPubkey: manager.publicKey,
    newAccountPubkey: token_pool.publicKey,
    programId: programId.publicKey,
  });

  treasury = Keypair.generate();
  [vault, _vault_bump] = await PublicKey.findProgramAddress(
    [Buffer.from("pool"), token_pool.publicKey.toBuffer()],
    programId.publicKey
  );
  const treasury_account_inst = SystemProgram.createAccount({
    space: 0,
    lamports: await connection.getMinimumBalanceForRentExemption(0),
    fromPubkey: manager.publicKey,
    newAccountPubkey: treasury.publicKey,
    programId: programId.publicKey,
  });
  const transaction_inst = new TransactionInstruction({
    keys: [
      { pubkey: manager.publicKey, isSigner: true, isWritable: true },
      { pubkey: vault, isSigner: false, isWritable: false },
      { pubkey: target_token.publicKey, isSigner: false, isWritable: false },
      { pubkey: token_pool.publicKey, isSigner: false, isWritable: true },
      { pubkey: treasury.publicKey, isSigner: false, isWritable: false },
      { pubkey: SYSVAR_RENT_PUBKEY, isSigner: false, isWritable: false },
      { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
    ],
    programId: programId.publicKey,
    data: Buffer.from(serialize(schema, value)),
  });
  const tx = new Transaction();
  tx.add(
    treasury_account_inst,
    token_members_list_inst,
    token_pool_account_inst,
    transaction_inst
  );
  await sendAndConfirmTransaction(connection, tx, [
    manager,
    token_members_list,
    token_pool,
    treasury,
  ]);

  const token_pool_data: Buffer = (await get_account_data(token_pool.publicKey))
    .data;
  const pool_data: TokenPool = TOKEN_POOL_LAYOUT.decode(token_pool_data);
  assert.equal(pool_data.targetAmount.toString(), "5");
  assert.equal(pool_data.minimumAmount.toString(), "1");
  pool_data.manager.equals(manager.publicKey);
  pool_data.targetToken.equals(target_token.publicKey);
  pool_data.treasury.equals(treasury.publicKey);
  assert.equal(pool_data.poolMemberList.members.length, max_members);
};

main().then(
  () => process.exit(),
  (err) => {
    console.log(err);
    process.exit(-1);
  }
);
