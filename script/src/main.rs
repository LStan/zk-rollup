use clap::Parser;
use onchain_types::CommittedValues;
use solana_sdk::{
    account::{Account, AccountSharedData, WritableAccount},
    hash::Hash,
    instruction::{AccountMeta, Instruction, InstructionError},
    loader_v4::{self, LoaderV4State, LoaderV4Status},
    native_token::LAMPORTS_PER_SOL,
    pubkey::Pubkey,
    rent::Rent,
    signature::Keypair,
    signer::Signer,
    system_instruction, system_program,
    transaction::Transaction,
};
use sp1_sdk::{include_elf, HashableKey, ProverClient, SP1Stdin};
use std::{
    fs::File,
    io::{Read, Write},
    vec,
};
use svm_runner_types::{hash_state, ExecutionInput, RampTx, RollupState};

pub const ZK_SVM_ELF: &[u8] = include_elf!("zk-svm-program");

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(long)]
    execute: bool,

    #[clap(long)]
    prove: bool,

    #[clap(long)]
    input: Option<Vec<u8>>,

    #[clap(long, short, default_value = "./sp1-proof.bin")]
    sp1_output_path: String,

    #[clap(long, short, default_value = "./onchain-commit.bin")]
    onchain_commit_path: String,

    #[clap(long, short, default_value = "./onchain-proof.bin")]
    onchain_proof_path: String,
}

// #[derive(Debug, BorshSerialize, BorshDeserialize)]
// struct OnChainProof {
//     pub public_values: Vec<u8>,
//     pub proof: Vec<u8>,
// }

fn main() {
    let args = Args::parse();

    if args.execute == args.prove {
        eprintln!("Error: You must specify either --execute or --prove");
        std::process::exit(1);
    }

    // Default to test input if user does not provide
    let input = if let Some(input) = args.input {
        bincode::deserialize(&input).unwrap()
    } else {
        create_test_input()
    };

    // let bytes = bincode::serialize(&input).unwrap();
    // let input = create_test_input();

    let client = ProverClient::from_env();
    let mut stdin = SP1Stdin::new();
    stdin.write(&input);

    if args.execute {
        // Execute the program
        let (output, report) = client.execute(ZK_SVM_ELF, &stdin).run().unwrap();
        println!("Program executed successfully.");

        // println!("output buffer: {}", output.raw());

        // Record the number of cycles executed.
        println!("Number of cycles: {}", report.total_instruction_count());

        let mut file =
            std::fs::File::create(args.onchain_commit_path).expect("failed to open file");
        file.write_all(&output.to_vec()).unwrap();

        // let data: CommittedValues = output.read();
        // println!("Committed values: {:?}", data);
    } else {
        println!("Initial state hash: {}", hash_state(input.accounts));

        // Setup the program for proving.
        let (pk, vk) = client.setup(ZK_SVM_ELF);
        println!("Verifying key: {}", vk.bytes32());

        println!("Starting proof generation...");
        let mut proof = client
            .prove(&pk, &stdin)
            .groth16()
            .run()
            .expect("failed to generate proof");
        proof
            .save(args.sp1_output_path)
            .expect("failed to save proof");

        let mut file =
            std::fs::File::create(args.onchain_commit_path).expect("failed to open file");
        file.write_all(&proof.public_values.to_vec()).unwrap();

        let mut file = std::fs::File::create(args.onchain_proof_path).expect("failed to open file");
        file.write_all(&proof.bytes()).unwrap();

        // let onchain_proof = OnChainProof {
        //     public_values: proof.public_values.to_vec(),
        //     proof: proof.bytes(),
        // };
        // let serialized_data = borsh::to_vec(&onchain_proof).unwrap();

        // let mut file = std::fs::File::create(args.onchain_output_path).expect("failed to open file");
        // file.write_all(&serialized_data).unwrap();
        // bincode
        //     ::serialize_into(
        //         std::fs::File::create(args.onchain_output_path).expect("failed to open file"),
        //         &onchain_proof
        //     )
        //     .unwrap();

        let commit: CommittedValues = proof.public_values.read();
        println!("Final state hash: {:?}", commit.output);

        println!("Successfully generated proof!");

        // Verify the proof.
        // client.verify(&proof, &vk).expect("failed to verify proof");
        // println!("Successfully verified proof!");
    }
}

fn create_test_input() -> ExecutionInput {
    let kp_sender_bytes: Vec<u8> =
        serde_json::from_slice(include_bytes!("../../onchain/tests/keypairSender.json")).unwrap();
    let kp_sender = Keypair::from_bytes(&kp_sender_bytes).unwrap();

    let kp_receiver_bytes: Vec<u8> =
        serde_json::from_slice(include_bytes!("../../onchain/tests/keypairReceiver.json")).unwrap();
    let kp_receiver = Keypair::from_bytes(&kp_receiver_bytes).unwrap();
    let pk_receiver = kp_receiver.pubkey();
    let pk_sender = kp_sender.pubkey();

    let counter_program_id = Keypair::new().pubkey();
    let pk_counter = Keypair::new().pubkey();

    let path = "../counter-program/counter_program.so";
    let mut file = File::open(path).expect("file open failed");
    let mut elf_bytes = Vec::new();
    file.read_to_end(&mut elf_bytes).unwrap();
    let rent = Rent::default();
    let account_size = LoaderV4State::program_data_offset().saturating_add(elf_bytes.len());
    let mut program_account = AccountSharedData::new(
        rent.minimum_balance(account_size),
        account_size,
        &loader_v4::id(),
    );
    let state = get_state_mut(program_account.data_as_mut_slice()).unwrap();
    state.slot = 0;
    state.authority_address_or_next_version = Pubkey::new_unique();
    state.status = LoaderV4Status::Deployed;
    program_account.data_as_mut_slice()[LoaderV4State::program_data_offset()..]
        .copy_from_slice(&elf_bytes);

    ExecutionInput {
        accounts: RollupState(vec![
            (
                pk_sender,
                Account {
                    lamports: 0,
                    data: vec![],
                    owner: system_program::id(),
                    executable: false,
                    rent_epoch: 0,
                }
                .into(),
            ),
            (
                pk_receiver,
                Account {
                    lamports: 0,
                    data: vec![],
                    owner: system_program::id(),
                    executable: false,
                    rent_epoch: 0,
                }
                .into(),
            ),
            (counter_program_id, program_account),
            (
                pk_counter,
                Account {
                    lamports: 100000,
                    data: vec![0, 0, 0, 0],
                    owner: counter_program_id,
                    executable: false,
                    rent_epoch: 0,
                }
                .into(),
            ),
        ]),
        txs: vec![
            Transaction::new_signed_with_payer(
                &[system_instruction::transfer(
                    &pk_sender,
                    &pk_receiver,
                    LAMPORTS_PER_SOL,
                )],
                Some(&pk_sender),
                &[&kp_sender],
                Hash::new_from_array([7; 32]),
            ),
            Transaction::new_signed_with_payer(
                &[Instruction {
                    program_id: counter_program_id,
                    accounts: vec![AccountMeta::new(pk_counter, false)],
                    data: vec![],
                }],
                Some(&pk_sender),
                &[&kp_sender],
                Hash::new_from_array([7; 32]),
            ),
        ],
        ramp_txs: vec![RampTx {
            is_onramp: true,
            user: pk_sender,
            amount: 10 * LAMPORTS_PER_SOL,
        }],
    }
}

fn get_state_mut(data: &mut [u8]) -> Result<&mut LoaderV4State, InstructionError> {
    unsafe {
        let data = data
            .get_mut(0..LoaderV4State::program_data_offset())
            .ok_or(InstructionError::AccountDataTooSmall)?
            .try_into()
            .unwrap();
        Ok(std::mem::transmute::<
            &mut [u8; LoaderV4State::program_data_offset()],
            &mut LoaderV4State,
        >(data))
    }
}

/*
#[test]
fn test_convert_proof() {
    let proof = sp1_sdk::SP1ProofWithPublicValues::load("./sp1-proof.bin").unwrap();
    // println!("{:?}", proof);
    // println!("public values: {:?}", proof.public_values.to_vec());
    // println!("public values len: {}", proof.public_values.to_vec().len());
    // println!("proof len: {}", proof.bytes().len());

    // let input: Vec<u8> = proof.public_values.read();
    // println!("input: {:?}", input);

    // let output: svm_runner_types::ExecutionOutput = proof.public_values.read();

    // println!("output: {:?}", output);

    // let commit = CommittedValues {
    //     input: bincode::deserialize(&input).unwrap(),
    //     output,
    // };

    // let bytes = bincode::serialize(&commit).unwrap();

    // println!("commit: {:?}", bytes);
    // println!("input len: {}", input.len());
    // println!("output len: {}", output.to_bytes().len());
    // println!("bytes len: {}", bytes.len());

    let onchain_proof = OnChainProof {
        public_values: proof.public_values.to_vec(),
        proof: proof.bytes(),
    };
    println!("{:?}", onchain_proof);
    println!("public values len: {}", onchain_proof.public_values.len());
    println!("proof len: {}", onchain_proof.proof.len());

    let serialized_data = borsh::to_vec(&onchain_proof).unwrap();

    // let mut file = std::fs::File::create("./onchain-proof.bin").expect("failed to open file");
    // file.write_all(&serialized_data).unwrap();

    // let bytes = onchain_proof.serialize(serializers::binary::Context::default()).unwrap();

    // bincode
    //     ::serialize_into(
    //         std::fs::File::create("./onchain-proof.bin").expect("failed to open file"),
    //         &onchain_proof
    //     )
    //     .unwrap();
}
*/
