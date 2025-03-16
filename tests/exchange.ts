import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Exchange } from "../target/types/exchange";
import { BN } from "bn.js";
import { createMint, getAssociatedTokenAddressSync, getOrCreateAssociatedTokenAccount } from "@solana/spl-token";

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
    poolMint = anchor.web3.PublicKey.findProgramAddressSync([Buffer.from("pool"),pool.toBuffer(),Buffer.from("pool_mint")], program.programId)[0]
    poolAuthority = anchor.web3.PublicKey.findProgramAddressSync([Buffer.from("pool"),pool.toBuffer(),Buffer.from("authority")], program.programId)[0]


    //initialize pool token accounts
    tokenA = (await getOrCreateAssociatedTokenAccount(connection, creator, tokenAMint, poolAuthority, true)).address;
    tokenB = (await getOrCreateAssociatedTokenAccount(connection, creator, tokenBMint, poolAuthority, true)).address;

    //creator pool token receipt accounts
    poolTokenRecieptAccount = getAssociatedTokenAddressSync(poolMint, creator.publicKey, false);
  })

  it("test initialize pool ok", async () => {

    const txSig = await program.methods.initialize({
      tradeFeeNumerator: new BN(5),
      tradeFeeDenominator: new BN(100),
      ownerTradeFeeNumerator:new BN(2),
      ownerTradeFeeDenominator: new BN(100),
      ownerWithdrawFeeNumerator: new BN(1),
      ownerWithdrawFeeDenomiator: new BN(100),
    }).accounts({
      tokenA,
      tokenB,
      poolTokenRecieptAccount,
      creator: creator.publicKey
    }).signers([creator]).rpc({skipPreflight: true});
    

    console.log("Your transaction signature", txSig);
  });
});
