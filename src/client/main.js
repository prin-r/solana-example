/**
 * PriceDB
 *
 * @flow
 */

import {
  establishConnection,
  establishPayer,
  loadProgram,
  setPrice,
  setValidator,
  verifyAndSetPrice,
} from "./pricedb";

async function main() {
  console.log("Begin interaction with PriceDB program on solana");

  // Establish connection to the cluster
  await establishConnection();

  // Determine who pays for the fees
  await establishPayer();

  // Load the program if not already loaded
  await loadProgram();

  // setPrice to the price keeper account
  // await setPrice(
  //   '006300000000000000'
  // );

  // setValidator to the validator keeper account
  // await setValidator(
  //   '010200000001010101010101010101010101010101010101010101010101010101010101010202020202020202020202020202020202020202020202020202020202020202'
  // );

  // verifyAndSetPrice
  // await verifyAndSetPrice(
  //   // [2;32] + 886270
  //   '02680000000000000000000000000000000000000000000000000000000000000000000040000000000000000000000000000000000000000000000000000000000000058002020202020202020202020202020202020202020202020202020202020202026f360e0000000000'
  // );

  console.log("Success");
}

main()
  .catch((err) => {
    console.error(err);
  })
  .then(() => process.exit());
