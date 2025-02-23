#![no_main]
sp1_zkvm::entrypoint!(main);

use svm_runner::runner;
use svm_runner_types::{hash_state, CommittedValues, ExecutionInput};

pub fn main() {
    let input = sp1_zkvm::io::read::<ExecutionInput>();

    let output = runner(&input).unwrap();

    // Commit to the input and output
    let commit = CommittedValues {
        input,
        output: hash_state(output),
    };
    sp1_zkvm::io::commit(&commit);
}
