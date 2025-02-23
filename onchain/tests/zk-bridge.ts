import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { ZkBridge } from "../target/types/zk_bridge";
import kpSender from "./keypairSender.json";
import kpReceiver from "./keypairReceiver.json";
import * as fs from "fs";
import * as borsh from "borsh";

// Define the structure of OnChainProof in TypeScript
class OnChainProof {
  publicValues: Uint8Array;
  proof: Uint8Array;

  static schema: borsh.Schema = {
    struct: {
      publicValues: { array: { type: "u8" } },
      proof: { array: { type: "u8" } },
    },
  };
}

describe("zk-bridge", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.ZkBridge as Program<ZkBridge>;

  const initialStateHash = "EukGGeg2sN2tETkZQP4kPTQxJQU859P8j5JGNLBKSt87";
  const senderKeypair = anchor.web3.Keypair.fromSecretKey(
    Uint8Array.from(Buffer.from(kpSender))
  );
  const receiverKeypair = anchor.web3.Keypair.fromSecretKey(
    Uint8Array.from(Buffer.from(kpReceiver))
  );

  const PLATFORM_SEED_PREFIX = getConstant(program.idl, "platformSeedPrefix");
  const COMMIT_SEED_PREFIX = getConstant(program.idl, "commitSeedPrefix");
  const RAMP_SEED_PREFIX = getConstant(program.idl, "rampSeedPrefix");

  // Read the file containing the serialized data
  const filePath = "../script/onchain-proof.bin";
  const fileData = fs.readFileSync(filePath);

  // Deserialize the data using borsh
  const onchainProof = borsh.deserialize(
    OnChainProof.schema,
    fileData
  ) as OnChainProof;
  // const onchainProof = borsh.deserialize;

  // console.log("OnChainProof:", onchainProof);
  console.log("Public Values Length:", onchainProof.publicValues.length);
  console.log("Proof Length:", onchainProof.proof.length);

  it("It works!", async () => {
    const platformId = anchor.web3.PublicKey.unique();
    const [platformKey, _platformBump] =
      anchor.web3.PublicKey.findProgramAddressSync(
        [Buffer.from(PLATFORM_SEED_PREFIX), platformId.toBuffer()],
        program.programId
      );
    const [rampKey, _rampBump] = anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from(RAMP_SEED_PREFIX),
        platformId.toBuffer(),
        senderKeypair.publicKey.toBuffer(),
      ],
      program.programId
    );

    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(
        senderKeypair.publicKey,
        10 * anchor.web3.LAMPORTS_PER_SOL
      )
    );

    await program.methods
      .createPlatform({
        id: platformId,
        initialStateHash: Array.from(Buffer.from(initialStateHash)),
      })
      .accountsPartial({
        sequencer: senderKeypair.publicKey,
        platform: platformKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([senderKeypair])
      .rpc();

    await program.methods
      .addRampTx({
        isOnramp: true,
        amount: new anchor.BN(anchor.web3.LAMPORTS_PER_SOL),
      })
      .accountsPartial({
        ramper: senderKeypair.publicKey,
        ramp: rampKey,
        platform: platformKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([senderKeypair])
      .rpc();

    // Upload commit
    const [commitKey, _commitBump] =
      anchor.web3.PublicKey.findProgramAddressSync(
        [
          Buffer.from(COMMIT_SEED_PREFIX),
          platformId.toBuffer(),
          senderKeypair.publicKey.toBuffer(),
        ],
        program.programId
      );

    // let dataLeft = onchainProof.publicValues;
    // CHECK: subarray doesn't work with out this, need fix
    let dataLeft = new Uint8Array(onchainProof.publicValues.length);
    for (let i = 0; i < onchainProof.publicValues.length; i++) {
      dataLeft[i] = onchainProof.publicValues[i];
    }

    let offset = 0;
    while (dataLeft.length > 0) {
      console.log(`uploading`);
      const size = Math.min(dataLeft.length, 800);
      await program.methods
        .uploadCommit({
          commitSize: new anchor.BN(onchainProof.publicValues.length),
          offset: new anchor.BN(offset),
          commitData: Buffer.from(dataLeft.subarray(0, size)),
        })
        .accountsPartial({
          prover: senderKeypair.publicKey,
          commit: commitKey,
          platform: platformKey,
        })
        .signers([senderKeypair])
        .rpc();

      dataLeft = dataLeft.subarray(size);
      offset += size;
    }

    await program.methods
      .prove(Buffer.from(onchainProof.proof))
      .accountsPartial({
        prover: senderKeypair.publicKey,
        commit: commitKey,
        platform: platformKey,
      })
      .preInstructions([
        anchor.web3.ComputeBudgetProgram.setComputeUnitLimit({
          units: 1_400_000,
        }),
      ])
      .signers([senderKeypair])
      .rpc({ skipPreflight: true });
  });
});

function getConstant(
  idl: ZkBridge,
  name: ZkBridge["constants"][number]["name"]
): ArrayBuffer {
  return JSON.parse(
    idl.constants.find((constant) => constant.name === name).value
  );
}
