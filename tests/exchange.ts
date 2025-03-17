import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Exchange } from "../target/types/exchange";
import { BN } from "bn.js";
import {
  createMint,
  getAssociatedTokenAddressSync,
  getOrCreateAssociatedTokenAccount,
  MintLayout,
  mintTo,
} from "@solana/spl-token";
import { assert } from "chai";

describe("exchange", () => {
  // Configure the client to use the local cluster.
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  const payer = anchor.web3.Keypair.generate();
  const program = anchor.workspace.Exchange as Program<Exchange>;

  const newProvider = new anchor.AnchorProvider(
    provider.connection,
    new anchor.Wallet(payer),
    {}
  );
  anchor.setProvider(newProvider);

  new anchor.Program(program.idl as anchor.Idl, newProvider);

  const connection = provider.connection;
  const creator = anchor.web3.Keypair.generate();
  const base = 1000_000_000;
  const tradeFeeNumerator = new BN(5);
  const tradeFeeDenominator = new BN(100);
  const ownerTradeFeeNumerator = new BN(2);
  const ownerTradeFeeDenominator = new BN(100);
  const ownerWithdrawFeeNumerator = new BN(1);
  const ownerWithdrawFeeDenomiator = new BN(100);

  let tokenA: anchor.web3.PublicKey;
  let tokenB: anchor.web3.PublicKey;
  let tokenAMint: anchor.web3.PublicKey;
  let tokenBMint: anchor.web3.PublicKey;
  let creatorPoolTokenReceipt: anchor.web3.PublicKey;
  let userPoolTokenReceipt: anchor.web3.PublicKey;
  let poolFeeAccount: anchor.web3.PublicKey;
  let pool: anchor.web3.PublicKey;
  let poolMint: anchor.web3.PublicKey;
  let poolAuthority: anchor.web3.PublicKey;

  before(async () => {
    // airdrops
    const airdropSigPayer = await connection.requestAirdrop(
      payer.publicKey,
      1_000_000_000
    );
    await connection.confirmTransaction(airdropSigPayer, "finalized");

    const airdropSigCreator = await connection.requestAirdrop(
      creator.publicKey,
      1_000_000_000
    );
    await connection.confirmTransaction(airdropSigCreator, "finalized");

    console.log(airdropSigPayer, airdropSigCreator);

    //initialize mints
    tokenAMint = await createMint(
      connection,
      creator,
      creator.publicKey,
      null,
      9
    );
    tokenBMint = await createMint(
      connection,
      creator,
      creator.publicKey,
      null,
      9
    );

    //pdas
    pool = anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from("pool"),
        tokenAMint.toBuffer(),
        tokenBMint.toBuffer(),
        creator.publicKey.toBuffer(),
      ],
      program.programId
    )[0];
    poolAuthority = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("pool"), pool.toBuffer(), Buffer.from("authority")],
      program.programId
    )[0];

    //pool mint
    poolMint = await createMint(
      connection,
      creator,
      poolAuthority,
      poolAuthority,
      9
    );

    //initialize pool token accounts
    tokenA = (
      await getOrCreateAssociatedTokenAccount(
        connection,
        creator,
        tokenAMint,
        poolAuthority,
        true
      )
    ).address;
    tokenB = (
      await getOrCreateAssociatedTokenAccount(
        connection,
        creator,
        tokenBMint,
        poolAuthority,
        true
      )
    ).address;

    await mintTo(connection, payer, tokenAMint, tokenA, creator, 1000 * base);
    await mintTo(connection, payer, tokenBMint, tokenB, creator, 1000 * base);

    //fee and creator pool token receipt accounts
    poolFeeAccount = getAssociatedTokenAddressSync(
      poolMint,
      poolAuthority,
      true
    );
    creatorPoolTokenReceipt = (
      await getOrCreateAssociatedTokenAccount(
        connection,
        creator,
        poolMint,
        creator.publicKey,
        false
      )
    ).address;
    userPoolTokenReceipt = (
      await getOrCreateAssociatedTokenAccount(
        connection,
        payer,
        poolMint,
        payer.publicKey,
        false
      )
    ).address;
  });

  it("test initialize pool ok", async () => {
    const txSig = await program.methods
      .initialize({
        tradeFeeNumerator,
        tradeFeeDenominator,
        ownerTradeFeeNumerator,
        ownerTradeFeeDenominator,
        ownerWithdrawFeeNumerator,
        ownerWithdrawFeeDenomiator,
      })
      .accountsPartial({
        tokenA,
        tokenB,
        poolMint,
        poolFeeAccount,
        userPoolTokenReceipt: creatorPoolTokenReceipt,
        creator: creator.publicKey,
      })
      .signers([creator])
      .rpc({ skipPreflight: true });

    const poolState = await program.account.pool.fetch(pool);
    assert.deepEqual(poolState.creator, creator.publicKey);
    assert.deepEqual(poolState.feeAccount, poolFeeAccount);
    assert.deepEqual(poolState.mint, poolMint);
    assert.deepEqual(poolState.tokenA, tokenA);
    assert.deepEqual(poolState.tokenB, tokenB);
    assert.deepEqual(poolState.tokenAMint, tokenAMint);
    assert.deepEqual(poolState.tokenBMint, tokenBMint);
    assert.equal(
      Number(poolState.fees.ownerTradeFeeNumerator),
      Number(ownerTradeFeeNumerator)
    );
    assert.equal(
      Number(poolState.fees.ownerTradeFeeDenominator),
      Number(ownerTradeFeeDenominator)
    );
    assert.equal(
      Number(poolState.fees.tradeFeeNumerator),
      Number(tradeFeeNumerator)
    );
    assert.equal(
      Number(poolState.fees.tradeFeeDenominator),
      Number(tradeFeeDenominator)
    );
    assert.equal(
      Number(poolState.fees.ownerWithdrawFeeNumerator),
      Number(ownerWithdrawFeeNumerator)
    );
    assert.equal(
      Number(poolState.fees.ownerWithdrawFeeDenomiator),
      Number(ownerWithdrawFeeDenomiator)
    );

    const poolMintInfo = await connection.getAccountInfo(poolMint);
    const poolMintData = MintLayout.decode(new Uint8Array(poolMintInfo.data));
    assert.equal(Number(poolMintData.supply), 1000_000_000);

    console.log("Your transaction signature", txSig);
  });

  it("test deposit all tokens ok", async () => {
    const tokenAAmount = await getTokenAmount(connection, tokenA);
    const tokenBAmount = await getTokenAmount(connection, tokenB);

    const poolMintInfo = await connection.getAccountInfo(poolMint);
    const poolMintData = MintLayout.decode(new Uint8Array(poolMintInfo.data));

    const poolTokenSupply = Number(poolMintData.supply);
    const maxTokenA = 100 * base;
    const maxTokenB = 100 * base;

    const userTokenAAccount = (
      await getOrCreateAssociatedTokenAccount(
        connection,
        creator,
        tokenAMint,
        payer.publicKey,
        true
      )
    ).address;
    const userTokenBAccount = (
      await getOrCreateAssociatedTokenAccount(
        connection,
        creator,
        tokenBMint,
        payer.publicKey,
        true
      )
    ).address;

    await mintTo(
      connection,
      payer,
      tokenAMint,
      userTokenAAccount,
      creator,
      200 * base
    );
    await mintTo(
      connection,
      payer,
      tokenBMint,
      userTokenBAccount,
      creator,
      200 * base
    );

    const minPoolTokens = Math.min(
      (maxTokenA * poolTokenSupply) / tokenAAmount,
      (maxTokenB * poolTokenSupply) / tokenBAmount
    );
    const txSig = await program.methods
      .depositAllTokensIn(
        new BN(minPoolTokens),
        new BN(maxTokenA),
        new BN(maxTokenB)
      )
      .accountsPartial({
        pool,
        poolAuthority,
        poolMint,
        poolTokenAAccount: tokenA,
        poolTokenBAccount: tokenB,
        poolTokenFeeAccount: poolFeeAccount,
        userPoolTokenReceipt,
        userTokenAAccount,
        userTokenBAccount,
        user: payer.publicKey,
        creator: creator.publicKey,
      })
      .signers([payer])
      .rpc();

    const userPoolTokenAmount = await getTokenAmount(
      connection,
      userPoolTokenReceipt
    );
    const newTokenAAmount = await getTokenAmount(connection, userTokenAAccount);
    const newTokenBAmount = await getTokenAmount(connection, userTokenBAccount);

    assert.equal(userPoolTokenAmount, minPoolTokens);

    assert(newTokenAAmount <= maxTokenA);
    assert(newTokenBAmount <= maxTokenB);

    console.log("Your transaction signature", txSig);
  });

  it("test withdraw single tokens out ok", async () => {
    const tokenAWithdrawAmount = 50 * base;
    const userTokenAAccount = (
      await getOrCreateAssociatedTokenAccount(
        connection,
        creator,
        tokenAMint,
        payer.publicKey,
        true
      )
    ).address;
    const oldUserTokenAAmount = await getTokenAmount(
      connection,
      userTokenAAccount
    );
    const oldUserPoolTokenAmount = await getTokenAmount(
      connection,
      userPoolTokenReceipt
    );

    const poolMintInfo = await connection.getAccountInfo(poolMint);
    const poolMintData = MintLayout.decode(new Uint8Array(poolMintInfo.data));

    const poolTokenSupply = Number(poolMintData.supply);
    const tokenAAmount = await getTokenAmount(connection, tokenA);
    const poolTokenPropotionalToWithdrawAmount =
      poolTokenSupply *
      (1 - Math.sqrt((tokenAAmount - tokenAWithdrawAmount) / tokenAAmount));

    const txSig = await program.methods
      .withdrawSingleTokenOut(new BN(tokenAWithdrawAmount))
      .accountsPartial({
        pool,
        poolAuthority,
        poolMint,
        poolTokenFeeAccount: poolFeeAccount,
        userPoolTokenReceipt,
        poolTokenAAccount: tokenA,
        poolTokenBAccount: tokenB,
        userSourceTokenAccount: userTokenAAccount,
        user: payer.publicKey,
        sourceMint: tokenAMint,
      })
      .signers([payer])
      .rpc();
    const newUserTokenAAmount = await getTokenAmount(
      connection,
      userTokenAAccount
    );
    const newUserPoolTokenAmount = await getTokenAmount(
      connection,
      userPoolTokenReceipt
    );

    assert.equal(
      newUserTokenAAmount,
      Math.round(oldUserTokenAAmount + tokenAWithdrawAmount)
    );
    assert.equal(
      newUserPoolTokenAmount,
      Math.round(oldUserPoolTokenAmount - poolTokenPropotionalToWithdrawAmount)
    );

    console.log("Your transaction signature", txSig);
  });

  it("test swap tokens ok", async () => {
    const tokenASwapAmount = 20 * base;
    const userTokenAAccount = (
      await getOrCreateAssociatedTokenAccount(
        connection,
        creator,
        tokenAMint,
        payer.publicKey,
        true
      )
    ).address;
    const userTokenBAccount = (
      await getOrCreateAssociatedTokenAccount(
        connection,
        creator,
        tokenBMint,
        payer.publicKey,
        true
      )
    ).address;

    const oldPoolTokenAAmount = await getTokenAmount(connection, tokenA);
    const oldPoolTokenBAmount = await getTokenAmount(connection, tokenB);

    const ownerFee =
      (tokenASwapAmount * Number(ownerTradeFeeNumerator)) /
      Number(ownerTradeFeeDenominator);
    const tradingFee =
      (tokenASwapAmount * Number(tradeFeeNumerator)) /
      Number(tradeFeeDenominator);
    const tokenASwapAmountAfterFee = tokenASwapAmount - (ownerFee + tradingFee);

    const expectedTokenBSwapAmount =
      oldPoolTokenBAmount -
      (oldPoolTokenAAmount * oldPoolTokenBAmount) /
        (oldPoolTokenAAmount + tokenASwapAmountAfterFee);

    const oldUserTokenBAmount = await getTokenAmount(
      connection,
      userTokenBAccount
    );
    const txSig = await program.methods
      .swap(new BN(tokenASwapAmount))
      .accountsPartial({
        pool,
        poolAuthority,
        poolMint,
        poolTokenAAccount: tokenA,
        poolTokenBAccount: tokenB,
        poolTokenFeeAccount: poolFeeAccount,
        userSourceTokenAccount: userTokenAAccount,
        userDestinationTokenAccount: userTokenBAccount,
        user: payer.publicKey,
        creator: creator.publicKey,
      })
      .signers([payer])
      .rpc();

    const newPoolTokenAAmount = await getTokenAmount(connection, tokenA);
    const newUserTokenBAmount = await getTokenAmount(
      connection,
      userTokenBAccount
    );

    assert.equal(newPoolTokenAAmount, oldPoolTokenAAmount + tokenASwapAmount);
    assert.equal(
      newUserTokenBAmount,
      Math.floor(oldUserTokenBAmount + expectedTokenBSwapAmount)
    );

    console.log("Your transaction signature", txSig);
  });

  it("test deposit single token ok", async () => {
    const tokenBDepositAmount = 40 * base;
    const userTokenBAccount = (
      await getOrCreateAssociatedTokenAccount(
        connection,
        creator,
        tokenBMint,
        payer.publicKey,
        true
      )
    ).address;
    // P' = P * [sqrt((A'+ A) / A) - 1]

    const oldUserTokenBAmount = await getTokenAmount(
      connection,
      userTokenBAccount
    );
    const oldUserPoolTokenAmount = await getTokenAmount(
      connection,
      userPoolTokenReceipt
    );

    const tokenBAmount = await getTokenAmount(connection, tokenB);
    const poolMintInfo = await connection.getAccountInfo(poolMint);
    const poolMintData = MintLayout.decode(new Uint8Array(poolMintInfo.data));

    const poolTokenSupply = Number(poolMintData.supply);
    const poolTokenPropotionalToDepositAmount =
      poolTokenSupply *
      (Math.sqrt((tokenBAmount + tokenBDepositAmount) / tokenBAmount) - 1);

    const txSig = await program.methods
      .depositSingleToken(new BN(tokenBDepositAmount))
      .accountsPartial({
        pool,
        poolAuthority,
        poolMint,
        poolTokenAAccount: tokenA,
        poolTokenBAccount: tokenB,
        userPoolTokenReceipt,
        userSourceTokenAccount: userTokenBAccount,
        user: payer.publicKey,
        sourceMint: tokenBMint,
      })
      .signers([payer])
      .rpc();

    const newUserTokenBAmount = await getTokenAmount(
      connection,
      userTokenBAccount
    );
    const newUserPoolTokenAmount = await getTokenAmount(
      connection,
      userPoolTokenReceipt
    );

    assert.equal(
      newUserTokenBAmount,
      oldUserTokenBAmount - tokenBDepositAmount
    );
    assert.equal(
      newUserPoolTokenAmount - oldUserPoolTokenAmount,
      Math.round(poolTokenPropotionalToDepositAmount)
    );

    console.log("Your transaction signature", txSig);
  });
});

async function getTokenAmount(
  connection: anchor.web3.Connection,
  token: anchor.web3.PublicKey
) {
  const tokenInfo = await connection.getTokenAccountBalance(token);
  return Number(tokenInfo.value.amount);
}
