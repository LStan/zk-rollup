use solana_bpf_loader_program::syscalls::create_program_runtime_environment_v1;
use solana_compute_budget::compute_budget::ComputeBudget;
use solana_program_runtime::loaded_programs::{BlockRelation, ForkGraph, ProgramCacheEntry};
use solana_sdk::{
    account::{AccountSharedData, ReadableAccount, WritableAccount},
    clock::Slot,
    feature_set::FeatureSet,
    fee::FeeStructure,
    hash::Hash,
    native_loader,
    pubkey::Pubkey,
    rent_collector::RentCollector,
    transaction::{self, SanitizedTransaction, TransactionError},
};

use solana_svm::{
    account_loader::CheckedTransactionDetails,
    transaction_processing_callback::TransactionProcessingCallback,
    transaction_processing_result::ProcessedTransaction,
    transaction_processor::{
        TransactionBatchProcessor, TransactionProcessingConfig, TransactionProcessingEnvironment,
    },
};

use solana_svm_transaction::svm_message::SVMMessage;
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, RwLock},
};
use svm_runner_types::{ExecutionInput, RollupState};

pub(crate) struct MockForkGraph {}

impl ForkGraph for MockForkGraph {
    fn relationship(&self, _a: Slot, _b: Slot) -> BlockRelation {
        BlockRelation::Unknown
    }
}

pub(crate) struct MockAccountLoader {
    pub account_shared_data: Arc<RwLock<HashMap<Pubkey, AccountSharedData>>>,
}

impl TransactionProcessingCallback for MockAccountLoader {
    fn get_account_shared_data(&self, pubkey: &Pubkey) -> Option<AccountSharedData> {
        self.account_shared_data
            .read()
            .unwrap()
            .get(pubkey)
            .cloned()
    }

    fn account_matches_owners(&self, account: &Pubkey, owners: &[Pubkey]) -> Option<usize> {
        self.get_account_shared_data(account)
            .and_then(|account| owners.iter().position(|key| account.owner().eq(key)))
    }

    fn add_builtin_account(&self, name: &str, program_id: &Pubkey) {
        let account_data = native_loader::create_loadable_account_with_fields(name, (5000, 0));
        self.account_shared_data
            .write()
            .unwrap()
            .insert(*program_id, account_data);
    }
}

pub fn runner(input: &ExecutionInput) -> Result<RollupState, TransactionError> {
    let mut account_shared_data = HashMap::<Pubkey, AccountSharedData>::new();

    for (pk, account) in &input.accounts.0 {
        account_shared_data.insert(*pk, account.clone());
    }

    // Process ramp txs
    for tx in &input.ramp_txs {
        let account = account_shared_data.get_mut(&tx.user).unwrap();

        if tx.is_onramp {
            account.set_lamports(account.lamports() + tx.amount);
        } else {
            account.set_lamports(account.lamports() - tx.amount);
        }
    }

    let account_loader = MockAccountLoader {
        account_shared_data: Arc::new(RwLock::new(account_shared_data)),
    };

    let fork_graph = Arc::new(RwLock::new(MockForkGraph {}));

    let processor = TransactionBatchProcessor::<MockForkGraph>::new(
        /* slot */ 1,
        /* epoch */ 1,
        Arc::downgrade(&fork_graph),
        Some(Arc::new(
            create_program_runtime_environment_v1(
                &FeatureSet::all_enabled(),
                &ComputeBudget::default(),
                false,
                false,
            )
            .unwrap(),
        )),
        None,
    );

    processor.add_builtin(
        &account_loader,
        solana_system_program::id(),
        "system_program",
        ProgramCacheEntry::new_builtin(
            0,
            b"system_program".len(),
            solana_system_program::system_processor::Entrypoint::vm,
        ),
    );

    let mut svm_transactions: Vec<SanitizedTransaction> = Vec::new();

    for tx in &input.txs {
        let sanitized_transaction =
            SanitizedTransaction::try_from_legacy_transaction(tx.clone(), &HashSet::new()).unwrap();
        svm_transactions.push(sanitized_transaction);
    }

    let fee_structure = FeeStructure::default();
    let rent_collector = RentCollector::default();

    // let processing_environment = TransactionProcessingEnvironment::default();
    let processing_environment = TransactionProcessingEnvironment {
        blockhash: Hash::default(),
        blockhash_lamports_per_signature: fee_structure.lamports_per_signature,
        epoch_total_stake: 0,
        feature_set: Arc::new(FeatureSet::all_enabled()),
        fee_lamports_per_signature: fee_structure.lamports_per_signature,
        rent_collector: Some(&rent_collector),
    };

    let processing_config = TransactionProcessingConfig {
        compute_budget: Some(ComputeBudget::default()),
        ..Default::default()
    };

    let results = processor.load_and_execute_sanitized_transactions(
        &account_loader,
        &svm_transactions,
        get_transaction_check_results(svm_transactions.len(), fee_structure.lamports_per_signature),
        &processing_environment,
        &processing_config,
    );

    for (tx_index, processed_transaction) in results.processing_results.iter().enumerate() {
        let sanitized_transaction = &svm_transactions[tx_index];

        match processed_transaction {
            Ok(ProcessedTransaction::Executed(executed_transaction)) => {
                for (index, (pubkey, account_data)) in executed_transaction
                    .loaded_transaction
                    .accounts
                    .iter()
                    .enumerate()
                {
                    if sanitized_transaction.is_writable(index) {
                        account_loader
                            .account_shared_data
                            .write()
                            .unwrap()
                            .insert(*pubkey, account_data.clone());
                    }
                }
            }
            Ok(ProcessedTransaction::FeesOnly(fees_only_transaction)) => {
                return Err(fees_only_transaction.load_error.clone())
            }
            Err(err) => return Err(err.clone()),
        }
    }

    Ok(RollupState(
        input
            .accounts
            .0
            .iter()
            .map(|state| {
                (
                    state.0.clone(),
                    account_loader
                        .account_shared_data
                        .read()
                        .unwrap()
                        .get(&state.0)
                        .unwrap()
                        .clone(),
                )
            })
            .collect(),
    ))
}

pub(crate) fn get_transaction_check_results(
    len: usize,
    lamports_per_signature: u64,
) -> Vec<transaction::Result<CheckedTransactionDetails>> {
    vec![transaction::Result::Ok(CheckedTransactionDetails::new(None, lamports_per_signature)); len]
}

#[cfg(test)]
mod tests {
    use solana_sdk::{
        account::Account, native_token::LAMPORTS_PER_SOL, signature::Keypair, signer::Signer,
        system_instruction, transaction::Transaction,
    };
    use svm_runner_types::RampTx;

    use super::*;

    #[test]
    fn test_runner() {
        let test_input = create_test_input();
        let result = runner(&test_input);
        assert!(result.is_ok());
        println!("result: {:?}", result.unwrap());
    }

    fn create_test_input() -> ExecutionInput {
        let kp_sender = Keypair::new();
        let kp_receiver = Keypair::new();
        let pk_receiver = kp_receiver.pubkey();
        let pk_sender = kp_sender.pubkey();

        ExecutionInput {
            accounts: RollupState(vec![
                (
                    pk_sender,
                    Account {
                        lamports: 0,
                        data: vec![],
                        owner: solana_system_program::id(),
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
                        owner: solana_system_program::id(),
                        executable: false,
                        rent_epoch: 0,
                    }
                    .into(),
                ),
            ]),
            txs: vec![Transaction::new_signed_with_payer(
                &[system_instruction::transfer(
                    &pk_sender,
                    &pk_receiver,
                    LAMPORTS_PER_SOL,
                )],
                Some(&pk_sender),
                &[&kp_sender],
                Hash::new_from_array([7; 32]),
            )],
            ramp_txs: vec![RampTx {
                is_onramp: true,
                user: pk_sender,
                amount: 10 * LAMPORTS_PER_SOL,
            }],
        }
    }
}
