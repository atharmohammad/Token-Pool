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
  AccountLayout,
  AuthorityType,
  createInitializeAccount3Instruction,
  createInitializeMint2Instruction,
  createMint,
  initializeMint2InstructionData,
  MintLayout,
  mintTo,
  TOKEN_PROGRAM_ID,
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
  TokenPoolInstructions,
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
  account: PublicKey
): Promise<AccountInfo<Buffer>> => {
  const account_after_buff = await connection.getAccountInfo(account);
  if (!account_after_buff) {
    console.log("Error!!");
    process.exit(-1);
  }
  return account_after_buff;
};

const max_members = 4;
export const description = "Monke NFT";
const TOKEN_POOL_SIZE =
  1 +
  8 +
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
const ESCROW_STATE_SIZE = 1 + 32 + 32 + 32 + 32 + 32 + 32 + 8;
const TOKEN_MEMBER_LIST_SIZE = 1 + 4 + (1 + 32 + 8 + 8 + 1 + 32) * max_members;

/* Since we are saying we would have maximum of 4 members in this token pool so we would initialize the space for max of 4 members */

let connection: Connection,
  programId: Keypair,
  manager: Keypair,
  token_pool: Keypair,
  token_members_list: Keypair,
  vault: PublicKey;
let _vault_bump: number,
  treasury: Keypair,
  escrow_state: Keypair,
  nft_escrow_state: Keypair,
  nft_listing_seller: Keypair,
  nft_listed: Keypair,
  nft_mint: Keypair,
  seller_nft_account: Keypair;

const main = async () => {
  const localenet = "http://127.0.0.1:8899";
  connection = new Connection(localenet);
  programId = await createKeypairFromFile(PROGRAM_KEYPAIR_PATH);
  console.log("Pinging ... !");
  await setupNFT();
  await listNft(); // list your nft on the platform
  await initialize();
  await airdrop_sol(connection, manager.publicKey);
  await addMember(manager, 0);
  const new_member = await createAccount(connection);
  await addMember(new_member, 1); // add new member in the pool
  await startSellEscrow(new_member, 1); // start escrow sale for new members share
  await buyShareEscrow(manager, new_member); // buy share
  await updateShare(manager, 0); // update share
  await buyNft();
};

/*** Amount are in lamports ***/

const buyNft = async () => {
  const member = await createAccount(connection);
  let value = getPayload(
    TokenPoolInstructions.AddMember,
    BigInt(2),
    BigInt(1),
    description,
    max_members
  );
  const transaction_inst_1 = new TransactionInstruction({
    keys: [
      { pubkey: member.publicKey, isSigner: true, isWritable: true },
      { pubkey: token_pool.publicKey, isSigner: false, isWritable: true },
      { pubkey: treasury.publicKey, isSigner: false, isWritable: true },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    programId: programId.publicKey,
    data: Buffer.from(serialize(schema, value)),
  });
  const tx1 = new Transaction();
  tx1.add(transaction_inst_1);
  await sendAndConfirmTransaction(connection, tx1, [member]);
  value = getPayload(
    TokenPoolInstructions.buyNft,
    BigInt(1),
    BigInt(1),
    description,
    max_members
  );
  const token_pool_data: Buffer = (await get_account_data(token_pool.publicKey))
    .data;
  const pool_data: TokenPool = TOKEN_POOL_LAYOUT.decode(token_pool_data);
  assert.equal(pool_data.stage, 1);
  assert.equal(pool_data.currentBalance.toString(), "5");
  const escrow_data_info = await get_account_data(nft_escrow_state.publicKey);
  const escrow_data: Escrow = ESCROW_LAYOUT.decode(escrow_data_info.data);

  const transaction_inst_2 = new TransactionInstruction({
    keys: [
      {
        pubkey: member.publicKey,
        isSigner: true,
        isWritable: true,
      },
      { pubkey: nft_escrow_state.publicKey, isSigner: false, isWritable: true },
      { pubkey: pool_data.vault, isSigner: false, isWritable: true },
      {
        pubkey: escrow_data.nft,
        isSigner: false,
        isWritable: true,
      },
      { pubkey: token_pool.publicKey, isSigner: false, isWritable: true },
      { pubkey: pool_data.treasury, isSigner: false, isWritable: true },
      { pubkey: escrow_data.seller, isSigner: false, isWritable: true },
      { pubkey: pool_data.targetToken, isSigner: false, isWritable: true },
      { pubkey: escrow_data.escrowVault, isSigner: false, isWritable: true },
      { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
    ],
    programId: programId.publicKey,
    data: Buffer.from(serialize(schema, value)),
  });
  const tx2 = new Transaction();
  tx2.add(transaction_inst_2);
  await sendAndConfirmTransaction(connection, tx2, [member]);

  const token_pool_data_2: Buffer = (
    await get_account_data(token_pool.publicKey)
  ).data;
  const pool_data_2: TokenPool = TOKEN_POOL_LAYOUT.decode(token_pool_data_2);
  console.log(pool_data_2);
};
const setupNFT = async () => {
  nft_mint = Keypair.generate();
  nft_listing_seller = await createAccount(connection);

  await createMint(
    connection,
    nft_listing_seller,
    nft_listing_seller.publicKey,
    nft_listing_seller.publicKey,
    0,
    nft_mint,
    undefined,
    TOKEN_PROGRAM_ID
  );
};
const listNft = async () => {
  const value = getPayload(
    TokenPoolInstructions.ListNFT,
    BigInt(5),
    BigInt(1),
    description,
    max_members
  );

  nft_escrow_state = Keypair.generate();
  const create_escrow_inst = SystemProgram.createAccount({
    space: ESCROW_STATE_SIZE,
    lamports: await connection.getMinimumBalanceForRentExemption(
      ESCROW_STATE_SIZE
    ),
    fromPubkey: nft_listing_seller.publicKey,
    newAccountPubkey: nft_escrow_state.publicKey,
    programId: programId.publicKey,
  });

  seller_nft_account = Keypair.generate();
  const create_token_acc_inst = SystemProgram.createAccount({
    space: AccountLayout.span,
    lamports: await connection.getMinimumBalanceForRentExemption(
      AccountLayout.span
    ),
    fromPubkey: nft_listing_seller.publicKey,
    newAccountPubkey: seller_nft_account.publicKey,
    programId: TOKEN_PROGRAM_ID,
  });
  const init_acc_inst = createInitializeAccount3Instruction(
    seller_nft_account.publicKey,
    nft_mint.publicKey,
    nft_listing_seller.publicKey,
    TOKEN_PROGRAM_ID
  );
  const tx1 = new Transaction();
  tx1.add(create_token_acc_inst, init_acc_inst);
  await sendAndConfirmTransaction(
    connection,
    tx1,
    [nft_listing_seller, seller_nft_account],
    undefined
  );
  await mintTo(
    connection,
    nft_listing_seller,
    nft_mint.publicKey,
    seller_nft_account.publicKey,
    nft_listing_seller.publicKey,
    1,
    undefined,
    undefined,
    TOKEN_PROGRAM_ID
  );
  const [vault, _bump] = await PublicKey.findProgramAddress(
    [Buffer.from("listnft"), seller_nft_account.publicKey.toBuffer()],
    programId.publicKey
  );
  const transaction_inst = new TransactionInstruction({
    keys: [
      {
        pubkey: nft_listing_seller.publicKey,
        isSigner: true,
        isWritable: true,
      },
      { pubkey: nft_escrow_state.publicKey, isSigner: false, isWritable: true },
      { pubkey: nft_mint.publicKey, isSigner: false, isWritable: true },
      { pubkey: vault, isSigner: false, isWritable: true },
      {
        pubkey: seller_nft_account.publicKey,
        isSigner: false,
        isWritable: true,
      },
      { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
    ],
    programId: programId.publicKey,
    data: Buffer.from(serialize(schema, value)),
  });
  const tx = new Transaction();
  tx.add(create_escrow_inst, transaction_inst);
  await sendAndConfirmTransaction(connection, tx, [
    nft_listing_seller,
    nft_escrow_state,
  ]);
  const escrow_data_info = await get_account_data(nft_escrow_state.publicKey);
  const escrow_data: Escrow = ESCROW_LAYOUT.decode(escrow_data_info.data);
  escrow_data.nft.equals(seller_nft_account.publicKey);
  escrow_data.nftMint.equals(nft_mint.publicKey);
  escrow_data.seller.equals(nft_listing_seller.publicKey);
  assert.equal(escrow_data.amount.toString(), "5");
  assert.equal(escrow_data.share, 100.0);
  assert.equal(escrow_data.stage, EscrowStage.Initialized);
  const nft_holding_account_data = await get_account_data(
    seller_nft_account.publicKey
  );
  const nft_holding_account = AccountLayout.decode(
    nft_holding_account_data.data
  );
  nft_holding_account.owner.equals(vault);
  const nft_mint_account_data = await get_account_data(
    nft_holding_account.mint
  );
  const nft_mint_account = MintLayout.decode(nft_mint_account_data.data);
  nft_mint_account.mintAuthority.equals(vault);
  nft_mint_account.freezeAuthority.equals(vault);
};

const updateShare = async (member: Keypair, index: number) => {
  const value = getPayload(
    TokenPoolInstructions.UpgradeShare,
    BigInt(1),
    BigInt(1),
    description,
    max_members
  );
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
  assert.equal(pool_data.currentBalance.toString(), "3");
  const treasury_data_buffer = await get_account_data(treasury.publicKey);
  assert.equal(
    treasury_data_buffer.lamports,
    (await connection.getMinimumBalanceForRentExemption(0)) + 3
  );
  assert.equal(pool_data.poolMemberList.members[index].share.toString(), "60");
  assert.equal(
    pool_data.poolMemberList.members[index].amountDeposited.toString(),
    "3"
  );
};

const buyShareEscrow = async (addedBuyer: Keypair, seller: Keypair) => {
  const value = getPayload(
    TokenPoolInstructions.BuyShare,
    BigInt(1),
    BigInt(1),
    description,
    max_members
  );
  const [escrow_vault, _escrow_vault_bump] = await PublicKey.findProgramAddress(
    [
      Buffer.from("escrow"),
      seller.publicKey.toBuffer(),
      token_pool.publicKey.toBuffer(),
    ],
    programId.publicKey
  );
  const transaction_inst = new TransactionInstruction({
    keys: [
      { pubkey: addedBuyer.publicKey, isSigner: true, isWritable: true },
      { pubkey: token_pool.publicKey, isSigner: false, isWritable: true },
      { pubkey: escrow_state.publicKey, isSigner: false, isWritable: true },
      { pubkey: escrow_vault, isSigner: false, isWritable: false },
      { pubkey: seller.publicKey, isSigner: false, isWritable: true },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    programId: programId.publicKey,
    data: Buffer.from(serialize(schema, value)),
  });
  const tx = new Transaction();
  tx.add(transaction_inst);
  await sendAndConfirmTransaction(connection, tx, [addedBuyer]);
  const token_pool_data: Buffer = (await get_account_data(token_pool.publicKey))
    .data;
  const pool_data: TokenPool = TOKEN_POOL_LAYOUT.decode(token_pool_data);
  assert.equal(
    pool_data.poolMemberList.members[0].amountDeposited.toString(),
    "2"
  );
  assert.equal(pool_data.poolMemberList.members[0].share.toString(), "40");
  pool_data.poolMemberList.members[0].memberKey.equals(addedBuyer.publicKey);
};

const startSellEscrow = async (member: Keypair, index: number) => {
  const value = getPayload(2, BigInt(1), BigInt(1), description, max_members); // only id needed , all other are placeholders
  escrow_state = Keypair.generate();
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
  pool_data.poolMemberList.members[index].memberKey.equals(escrow_vault);
  const escrow_data_buffer: Buffer = (
    await get_account_data(escrow_state.publicKey)
  ).data;
  const escrow_data: Escrow = ESCROW_LAYOUT.decode(escrow_data_buffer);
  assert.equal(escrow_data.stage, EscrowStage.Initialized);
  escrow_data.seller.equals(member.publicKey);
  escrow_data.escrowVault.equals(escrow_vault);
  escrow_data.nft.equals(pool_data.targetToken);
  assert.equal(
    escrow_data.share,
    pool_data.poolMemberList.members[index].share
  );
  assert.equal(escrow_data.amount, 1); // amount want for the share is 1
  pool_data.poolMemberList.members[index].escrow.equals(escrow_state.publicKey);
  assert.equal(
    pool_data.poolMemberList.members[index].shareStage,
    ShareStage.Escrowed
  );
};

const addMember = async (member: Keypair, index: number) => {
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
  const old_share = 100 / 5;

  if (index > 0) {
    assert.equal(
      pool_data.poolMemberList.members[0].share,
      old_share + pool_data.minimumExemptionShare * index
    );
  }
  if (index == 0)
    assert.equal(pool_data.poolMemberList.members[index].share, old_share);
  else
    assert.equal(
      pool_data.poolMemberList.members[index].share,
      old_share - pool_data.minimumExemptionShare
    );

  assert.equal(pool_data.currentBalance.toString(), (index + 1).toString());
  assert.equal(
    pool_data.poolMemberList.members[index].amountDeposited.toString(),
    "1"
  );
  assert.equal(
    pool_data.poolMemberList.members[index].accountType,
    AccountType.TokenPoolMember
  );
  member.publicKey.equals(pool_data.poolMemberList.members[index].memberKey);
  const treasury_data_buffer = await get_account_data(treasury.publicKey);
  assert.equal(
    treasury_data_buffer.lamports,
    (await connection.getMinimumBalanceForRentExemption(0)) + index + 1
  );
  assert.equal(
    pool_data.poolMemberList.members[index].shareStage,
    ShareStage.Hold
  );
  pool_data.poolMemberList.members[index].escrow.equals(PublicKey.default);
};

const initialize = async () => {
  const value = getPayload(
    0,
    BigInt(5),
    BigInt(1),
    description,
    max_members,
    "1.2"
  );
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
      { pubkey: nft_mint.publicKey, isSigner: false, isWritable: false }, // mint of the nft
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
  pool_data.targetToken.equals(nft_mint.publicKey);
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
