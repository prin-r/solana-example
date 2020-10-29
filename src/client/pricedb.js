// @flow

import {
  Account,
  Connection,
  BpfLoader,
  BPF_LOADER_PROGRAM_ID,
  PublicKey,
  LAMPORTS_PER_SOL,
  SystemProgram,
  TransactionInstruction,
  Transaction,
} from "@solana/web3.js";
import fs from "mz/fs";
import * as BufferLayout from "buffer-layout";

import { url, urlTls } from "../../url";
import { Store } from "./util/store";
import { newAccountWithLamports } from "./util/new-account-with-lamports";
import { sendAndConfirmTransaction } from "./util/send-and-confirm-transaction";

/**
 * Connection to the network
 */
let connection: Connection;

/**
 * Connection to the network
 */
let payerAccount: Account;

/**
 * PriceDB's program id
 */
let programId: PublicKey;

/**
 * The public key of the PriceDBKeeper account
 */
let pdbkPubkey: PublicKey;

/**
 * The public key of the ValidatorKeeper account
 */
let vkPubkey: PublicKey;

const pathToProgram = "dist/program/pricedb.so";

/**
 * Layout of the PriceDBKeeper account data
 */
const pdbkAccountDataLayout = BufferLayout.struct([
  BufferLayout.u8(""),
  BufferLayout.f64(""),
]);

/**
 * Layout of the ValidatorKeeper account data
 */
const vkAccountDataLayout = BufferLayout.struct([
  BufferLayout.u8(""),
  BufferLayout.u32(""),
  BufferLayout.blob(32, ""),
  BufferLayout.blob(32, ""),
]);

const pdbAccountDataLayout = BufferLayout.struct([
  BufferLayout.u32(""),

  BufferLayout.u32(""),
  BufferLayout.blob(8, ""),
  BufferLayout.nu64(""),
  BufferLayout.nu64(""),

  BufferLayout.u32(""),
  BufferLayout.blob(8, ""),
  BufferLayout.nu64(""),
  BufferLayout.nu64(""),

  BufferLayout.blob(32, ""),
]);

/**
 * Establish a connection to the cluster
 */
export async function establishConnection(): Promise<void> {
  connection = new Connection(url, "recent");
  const version = await connection.getVersion();
  console.log("Connection to cluster established:", url, version);
}

/**
 * Establish an account to pay for everything
 */
export async function establishPayer(): Promise<void> {
  if (!payerAccount) {
    let fees = 0;
    const { feeCalculator } = await connection.getRecentBlockhash();

    // Calculate the cost to load the program
    const data = await fs.readFile(pathToProgram);
    const NUM_RETRIES = 500; // allow some number of retries
    fees +=
      feeCalculator.lamportsPerSignature *
        (BpfLoader.getMinNumSignatures(data.length) + NUM_RETRIES) +
      (await connection.getMinimumBalanceForRentExemption(data.length));

    // Calculate the cost to fund the greeter account
    fees += await connection.getMinimumBalanceForRentExemption(
      pdbkAccountDataLayout.span
    );

    fees += await connection.getMinimumBalanceForRentExemption(
      vkAccountDataLayout.span
    );

    console.log(pdbkAccountDataLayout.span);
    console.log(vkAccountDataLayout.span);

    // Calculate the cost of sending the transactions
    fees += feeCalculator.lamportsPerSignature * 100; // wag

    // Fund a new payer via airdrop
    payerAccount = await newAccountWithLamports(connection, fees);
  }

  const lamports = await connection.getBalance(payerAccount.publicKey);
  console.log(
    "Using account",
    payerAccount.publicKey.toBase58(),
    "containing",
    lamports / LAMPORTS_PER_SOL,
    "Sol to pay for fees"
  );
}

/**
 * Load the PriceDB BPF program if not already loaded
 */
export async function loadProgram(): Promise<void> {
  const store = new Store();
  let pdbkPubkey;
  // Check if the program has already been loaded
  try {
    let config = await store.load("config.json");
    programId = new PublicKey(config.programId);
    pdbkPubkey = new PublicKey(config.greetedPubkey);
    await connection.getAccountInfo(programId);
    console.log("Program already loaded to account", programId.toBase58());
    return;
  } catch (err) {
    // try to load the program
  }

  // Load the program
  console.log("Loading std_reference_basic program...");
  const data = await fs.readFile(pathToProgram);
  const programAccount = new Account();
  await BpfLoader.load(
    connection,
    payerAccount,
    programAccount,
    data,
    BPF_LOADER_PROGRAM_ID
  );
  programId = programAccount.publicKey;
  console.log(
    "Program loaded to account",
    programId.toBase58(),
    " to be the PriceDBKeeper"
  );

  // Create an account
  const pdbkAccount = new Account();
  pdbkPubkey = pdbkAccount.publicKey;
  console.log("Creating account", pdbkPubkey.toBase58());
  const space = BufferLayout.struct([BufferLayout.u32("test")]);
  const lamports = await connection.getMinimumBalanceForRentExemption(
    pdbAccountDataLayout.span
  );
  const transaction = new Transaction().add(
    SystemProgram.createAccount({
      fromPubkey: payerAccount.publicKey,
      newAccountPubkey: pdbkPubkey,
      lamports,
      space,
      programId,
    })
  );
  await sendAndConfirmTransaction(
    "createAccount",
    connection,
    transaction,
    [payerAccount],
    pdbkAccount
  );

  // Save this info for next time
  await store.save("config.json", {
    url: urlTls,
    programId: programId.toBase58(),
    greetedPubkey: greetedPubkey.toBase58(),
  });
}

/**
 * Set price
 */
export async function setPrice(encodedCommand): Promise<void> {
  console.log("setPrice for ", pdbkPubkey.toBase58());
  const instruction = new TransactionInstruction({
    keys: [{ pubkey: pdbkPubkey, isSigner: false, isWritable: true }],
    programId,
    // Set price to be 99, encode with borsh
    data: Buffer.from(encodedCommand, "hex"),
  });
  await sendAndConfirmTransaction(
    "setPrice",
    connection,
    new Transaction().add(instruction),
    payerAccount
  );
}

/**
 * Set validators
 */
export async function setValidator(encodedCommand): Promise<void> {
  console.log("setValidator for ", vkPubkey.toBase58());
  const instruction = new TransactionInstruction({
    keys: [{ pubkey: vkPubkey, isSigner: false, isWritable: true }],
    programId,
    // Borsh encode of ValidatorKeeper that contain pubkeys [1;32] and [2;32]
    data: Buffer.from(encodedCommand, "hex"),
  });
  await sendAndConfirmTransaction(
    "setValidator",
    connection,
    new Transaction().add(instruction),
    payerAccount
  );
}

/**
 * Varify and set price
 */
export async function verifyAndSetPrice(encodedCommand): Promise<void> {
  console.log(
    "Verify and set price for ",
    pdbkPubkey.toBase58(),
    " and ",
    vkPubkey.toBase58()
  );
  const instruction = new TransactionInstruction({
    keys: [
      { pubkey: pdbkPubkey, isSigner: false, isWritable: true },
      { pubkey: vkPubkey, isSigner: false, isWritable: true },
    ],
    programId,
    data: Buffer.from(encodedCommand, "hex"),
  });
  await sendAndConfirmTransaction(
    "VerifyAndSetPrice",
    connection,
    new Transaction().add(instruction),
    payerAccount
  );
}

/**
 * Report the number of times the greeted account has been said hello to
 */
export async function reportHellos(): Promise<void> {
  const accountInfo = await connection.getAccountInfo(greetedPubkey);
  if (accountInfo === null) {
    throw "Error: cannot find the greeted account";
  }
  const info = pdbkAccountDataLayout.decode(Buffer.from(accountInfo.data));
  console.log(
    greetedPubkey.toBase58(),
    "has been greeted",
    info.numGreets.toString(),
    "times"
  );
}
