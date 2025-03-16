import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Exchange } from "../target/types/exchange";
import { BN } from "bn.js";
import { createMint, getAssociatedTokenAddressSync, getOrCreateAssociatedTokenAccount, MintLayout } from "@solana/spl-token";
import { assert } from "chai";

describe("exchange", () => {
  // Configure the client to use the local cluster.
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  const payer = anchor.web3.Keypair.generate();
  const program = anchor.workspace.Exchange as Program<Exchange>;

  const newProvider = new anchor.AnchorProvider(provider.connection, new anchor.Wallet(payer), {});
  anchor.setProvider(newProvider);

  new anchor.Program(program.idl as anchor.Idl, newProvider)

  const connection = provider.connection;
  const creator = payer;
  let tokenA: anchor.web3.PublicKey;
  let tokenB : anchor.web3.PublicKey;
  let tokenAMint : anchor.web3.PublicKey;
  let tokenBMint : anchor.web3.PublicKey;
  let poolTokenRecieptAccount : anchor.web3.PublicKey;
  let poolFeeAccount: anchor.web3.PublicKey;
  let pool : anchor.web3.PublicKey;
  let poolMint : anchor.web3.PublicKey;
  let poolAuthority : anchor.web3.PublicKey;

  before(async() => {
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

    console.log(airdropSigPayer, airdropSigCreator)

    //initialize mints
    tokenAMint = await createMint(connection, creator, creator.publicKey, null, 9);
    tokenBMint = await createMint(connection, creator, creator.publicKey, null, 9);

    //pdas
    pool = anchor.web3.PublicKey.findProgramAddressSync([Buffer.from("pool"),tokenAMint.toBuffer(),tokenBMint.toBuffer(),creator.publicKey.toBuffer()], program.programId)[0]
    poolAuthority = anchor.web3.PublicKey.findProgramAddressSync([Buffer.from("pool"),pool.toBuffer(),Buffer.from("authority")], program.programId)[0]

    //pool mint
    poolMint = await createMint(connection, creator, poolAuthority, poolAuthority, 9);

    //initialize pool token accounts
    tokenA = (await getOrCreateAssociatedTokenAccount(connection, creator, tokenAMint, poolAuthority, true)).address;
    tokenB = (await getOrCreateAssociatedTokenAccount(connection, creator, tokenBMint, poolAuthority, true)).address;

    //fee and creator pool token receipt accounts
    poolFeeAccount = getAssociatedTokenAddressSync(poolMint, poolAuthority, true);
    poolTokenRecieptAccount = (await getOrCreateAssociatedTokenAccount(connection, creator, poolMint, creator.publicKey, false)).address;
  })

  it("test initialize pool ok", async () => {
    const tradeFeeNumerator = new BN(5);
    const tradeFeeDenominator = new BN(100);
    const ownerTradeFeeNumerator = new BN(2);
    const ownerTradeFeeDenominator =  new BN(100);
    const ownerWithdrawFeeNumerator = new BN(1);
    const ownerWithdrawFeeDenomiator = new BN(100);

    const txSig = await program.methods.initialize({
      tradeFeeNumerator,
      tradeFeeDenominator,
      ownerTradeFeeNumerator,
      ownerTradeFeeDenominator,
      ownerWithdrawFeeNumerator,
      ownerWithdrawFeeDenomiator,
    }).accounts({
      tokenA,
      tokenB,
      poolMint,
      poolFeeAccount,
      poolTokenRecieptAccount,
      creator: creator.publicKey
    }).signers([creator]).rpc({skipPreflight: true});

    const poolState = await program.account.pool.fetch(pool);
    assert.deepEqual(poolState.creator, creator.publicKey);
    assert.deepEqual(poolState.feeAccount, poolFeeAccount);
    assert.deepEqual(poolState.mint, poolMint);
    assert.deepEqual(poolState.tokenA, tokenA);
    assert.deepEqual(poolState.tokenB, tokenB);
    assert.deepEqual(poolState.tokenAMint, tokenAMint);
    assert.deepEqual(poolState.tokenBMint, tokenBMint);
    assert.equal(Number(poolState.fees.ownerTradeFeeNumerator), Number(ownerTradeFeeNumerator));
    assert.equal(Number(poolState.fees.ownerTradeFeeDenominator), Number(ownerTradeFeeDenominator));
    assert.equal(Number(poolState.fees.tradeFeeNumerator), Number(tradeFeeNumerator));
    assert.equal(Number(poolState.fees.tradeFeeDenominator), Number(tradeFeeDenominator));
    assert.equal(Number(poolState.fees.ownerWithdrawFeeNumerator), Number(ownerWithdrawFeeNumerator));
    assert.equal(Number(poolState.fees.ownerWithdrawFeeDenomiator), Number(ownerWithdrawFeeDenomiator));

    const poolMintInfo = await connection.getAccountInfo(poolMint);
    const poolMintData = MintLayout.decode(new Uint8Array(poolMintInfo.data));
    assert.equal(Number(poolMintData.supply), 1000_000_000);

    console.log("Your transaction signature", txSig);
  });
});
