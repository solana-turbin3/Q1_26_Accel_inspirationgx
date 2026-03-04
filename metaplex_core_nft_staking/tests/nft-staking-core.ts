import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { NftStakingCore } from "../target/types/nft_staking_core";
import { SystemProgram } from "@solana/web3.js";
import { MPL_CORE_PROGRAM_ID } from "@metaplex-foundation/mpl-core";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  getAssociatedTokenAddressSync,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";

const MILLISECONDS_PER_DAY = 86400000;
const POINTS_PER_STAKED_NFT_PER_DAY = 10_000_000;
const FREEZE_PERIOD_IN_DAYS = 0;
const TIME_TRAVEL_IN_DAYS = 8;

describe("nft-staking-core", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.nftStakingCore as Program<NftStakingCore>;

  // Generate a keypair for the collection
  const collectionKeypair = anchor.web3.Keypair.generate();

  // Find the update authority for the collection (PDA)
  const updateAuthority = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("update_authority"), collectionKeypair.publicKey.toBuffer()],
    program.programId,
  )[0];

  // Generate a keypair for the nft asset
  const nftKeypair = anchor.web3.Keypair.generate();

  // Find the config account (PDA)
  const config = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("config"), collectionKeypair.publicKey.toBuffer()],
    program.programId,
  )[0];

  // Find the rewards mint account (PDA)
  const rewardsMint = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("rewards"), config.toBuffer()],
    program.programId,
  )[0];

  it("Create a collection", async () => {
    const collectionName = "Test Collection";
    const collectionUri = "https://example.com/collection";
    const tx = await program.methods
      .createCollection(collectionName, collectionUri)
      .accountsPartial({
        payer: provider.wallet.publicKey,
        collection: collectionKeypair.publicKey,
        updateAuthority,
        systemProgram: SystemProgram.programId,
        mplCoreProgram: MPL_CORE_PROGRAM_ID,
      })
      .signers([collectionKeypair])
      .rpc();
    console.log("\nYour transaction signature", tx);
    console.log("Collection address", collectionKeypair.publicKey.toBase58());
  });

  it("Mint an NFT", async () => {
    const nftName = "Test NFT";
    const nftUri = "https://example.com/nft";
    const tx = await program.methods
      .mintNft(nftName, nftUri)
      .accountsPartial({
        user: provider.wallet.publicKey,
        nft: nftKeypair.publicKey,
        collection: collectionKeypair.publicKey,
        updateAuthority,
        systemProgram: SystemProgram.programId,
        mplCoreProgram: MPL_CORE_PROGRAM_ID,
      })
      .signers([nftKeypair])
      .rpc();
    console.log("\nYour transaction signature", tx);
    console.log("NFT address", nftKeypair.publicKey.toBase58());
  });

  it("Initialize stake config", async () => {
    const tx = await program.methods
      .initializeConfig(POINTS_PER_STAKED_NFT_PER_DAY, FREEZE_PERIOD_IN_DAYS)
      .accountsPartial({
        admin: provider.wallet.publicKey,
        collection: collectionKeypair.publicKey,
        updateAuthority,
        config,
        rewardsMint,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .rpc();
    console.log("\nYour transaction signature", tx);
    console.log("Config address", config.toBase58());
    console.log("Points per staked NFT per day", POINTS_PER_STAKED_NFT_PER_DAY);
    console.log("Freeze period in days", FREEZE_PERIOD_IN_DAYS);
    console.log("Rewards mint address", rewardsMint.toBase58());
  });

  it("Stake an NFT", async () => {
    const tx = await program.methods
      .stake()
      .accountsPartial({
        user: provider.wallet.publicKey,
        updateAuthority,
        config,
        nft: nftKeypair.publicKey,
        collection: collectionKeypair.publicKey,
        systemProgram: SystemProgram.programId,
        mplCoreProgram: MPL_CORE_PROGRAM_ID,
      })
      .rpc();
    console.log("\nYour transaction signature", tx);
  });

  /**
   * Helper function to advance time with surfnet_timeTravel RPC method
   * @param params - Time travel params (absoluteEpoch, absoluteSlot, or absoluteTimestamp)
   */
  // async function advanceTime(params: {
  //   absoluteEpoch?: number;
  //   absoluteSlot?: number;
  //   absoluteTimestamp?: number;
  // }): Promise<void> {
  //   const rpcResponse = await fetch(provider.connection.rpcEndpoint, {
  //     method: "POST",
  //     headers: { "Content-Type": "application/json" },
  //     body: JSON.stringify({
  //       jsonrpc: "2.0",
  //       id: 1,
  //       method: "surfnet_timeTravel",
  //       params: [params],
  //     }),
  //   });

  //   const result = (await rpcResponse.json()) as { error?: any; result?: any };
  //   if (result.error) {
  //     throw new Error(`Time travel failed: ${JSON.stringify(result.error)}`);
  //   }

  //   await new Promise((resolve) => setTimeout(resolve, 1000));
  // }

  // it("Time travel to the future", async () => {
  //   // Advance time in milliseconds
  //   const currentTimestamp = Date.now();
  //   await advanceTime({
  //     absoluteTimestamp:
  //       currentTimestamp + TIME_TRAVEL_IN_DAYS * MILLISECONDS_PER_DAY,
  //   });
  //   console.log("\nTime traveled in days", TIME_TRAVEL_IN_DAYS);
  // });

  async function advanceTime(params: {
    absoluteEpoch?: number;
    absoluteSlot?: number;
    absoluteTimestamp?: number;
  }): Promise<void> {
    const target = params.absoluteTimestamp;

    const rpcResponse = await fetch(provider.connection.rpcEndpoint, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        jsonrpc: "2.0",
        id: 1,
        method: "surfnet_timeTravel",
        params: [{ absoluteTimestamp: target }],
      }),
    });

    const result = await rpcResponse.json();
    if (result.error) {
      throw new Error(`Time travel failed: ${result.error.message}`);
    }

    await new Promise((resolve) => setTimeout(resolve, 2000));

    // Wait for a few new slots to be produced at the new time
    const currentSlot = await provider.connection.getSlot();
    await provider.connection.confirmTransaction({
      signature: await provider.connection.requestAirdrop(
        provider.wallet.publicKey,
        1, // minimal lamports
      ),
      ...(await provider.connection.getLatestBlockhash()),
    });
  }

  it("Time travel to the future with detailed logs", async () => {
    // Read the Clock sysvar to get the REAL cluster unix_timestamp
    const CLOCK_SYSVAR = new anchor.web3.PublicKey(
      "SysvarC1ock11111111111111111111111111111111",
    );
    const clockAccount = await provider.connection.getAccountInfo(CLOCK_SYSVAR);

    if (!clockAccount) throw new Error("Cannot read Clock sysvar");

    // unix_timestamp is at offset 32, as i64 (8 bytes LE)
    const currentUnixTimestamp = Number(clockAccount.data.readBigInt64LE(32));
    console.log("Clock unix_timestamp:", currentUnixTimestamp);
    console.log(
      "Clock date:",
      new Date(currentUnixTimestamp * 1000).toISOString(),
    );

    // Jump 20 days forward — target in MILLISECONDS
    const target = currentUnixTimestamp * 1000 + 20 * 24 * 60 * 60 * 1000;
    console.log("Target timestamp (ms):", target);

    const rpcResponse = await fetch(provider.connection.rpcEndpoint, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        jsonrpc: "2.0",
        id: 1,
        method: "surfnet_timeTravel",
        params: [{ absoluteTimestamp: target }],
      }),
    });

    const result = await rpcResponse.json();
    console.log("RPC result:", JSON.stringify(result, null, 2));

    if (result.error) {
      throw new Error(`Time travel failed: ${result.error.message}`);
    }

    await new Promise((resolve) => setTimeout(resolve, 3000));
  });

  it("Verify time travel worked", async () => {
    const slot = await provider.connection.getSlot();
    const blockTime = await provider.connection.getBlockTime(slot);
    const epochInfo = await provider.connection.getEpochInfo();

    console.log("Post-travel slot:", slot);
    console.log("Post-travel epoch:", epochInfo.epoch);
    console.log("Post-travel blockTime:", blockTime);

    // Read Clock sysvar directly
    const CLOCK_SYSVAR = new anchor.web3.PublicKey(
      "SysvarC1ock11111111111111111111111111111111",
    );
    const clockAccount = await provider.connection.getAccountInfo(CLOCK_SYSVAR);
    if (clockAccount) {
      const postUnixTimestamp = Number(clockAccount.data.readBigInt64LE(32));
      console.log("Post-travel Clock unix_timestamp:", postUnixTimestamp);
      console.log(
        "Post-travel Clock date:",
        new Date(postUnixTimestamp * 1000).toISOString(),
      );
    }
  });

  it("Unstake an NFT", async () => {
    // Get the user rewards ATA account
    const userRewardsAta = getAssociatedTokenAddressSync(
      rewardsMint,
      provider.wallet.publicKey,
      false,
      TOKEN_PROGRAM_ID,
      ASSOCIATED_TOKEN_PROGRAM_ID,
    );
    const tx = await program.methods
      .unstake()
      .accountsPartial({
        user: provider.wallet.publicKey,
        updateAuthority,
        config,
        rewardsMint,
        userRewardsAta,
        nft: nftKeypair.publicKey,
        collection: collectionKeypair.publicKey,
        mplCoreProgram: MPL_CORE_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      })
      .rpc();
    console.log("\nYour transaction signature", tx);
    console.log(
      "User rewards balance",
      (await provider.connection.getTokenAccountBalance(userRewardsAta)).value
        .uiAmount,
    );
  });
});
