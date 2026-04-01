use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::{env, error::Error, fs, path::Path, sync::Arc};
use tokio::time::{sleep, Duration};

use miden_client::{
    account::{
        component::{BasicFungibleFaucet, BasicWallet},
        AccountBuilder, AccountComponent, AccountId, AccountStorageMode, AccountType, StorageSlot,
        StorageSlotName,
    },
    address::NetworkId,
    asset::TokenSymbol,
    assembly::{
        Assembler, CodeBuilder, DefaultSourceManager, Library, Module, ModuleKind,
        Path as AssemblyPath,
    },
    auth::{AuthFalcon512Rpo, AuthSecretKey, NoAuth},
    builder::ClientBuilder,
    keystore::FilesystemKeyStore,
    note::{create_p2id_note, NoteAttachment, NoteType},
    rpc::{Endpoint, GrpcClient},
    store::{AccountRecordData, TransactionFilter},
    transaction::{
        OutputNote, TransactionId, TransactionKernel, TransactionRequestBuilder, TransactionStatus,
    },
    Client, ClientError, Felt, Word,
};
use miden_client_sqlite_store::ClientBuilderSqliteExt;
use miden_protocol::account::AccountIdVersion;

const ARTIFACTS_PATH: &str = "artifacts/official_activity.json";
const COUNTER_SLOT: &str = "miden::tutorials::counter";
const COUNTER_ACCOUNT_CODE_PATH: &str = "masm/accounts/counter.masm";
const COUNTER_SCRIPT_PATH: &str = "masm/scripts/counter_script.masm";

#[derive(Debug, Serialize, Deserialize)]
struct ActivityArtifacts {
    network: String,
    alice_account_id: String,
    faucet_account_id: String,
    counter_contract_id: String,
    minted_note_count: usize,
    minted_note_amount: u64,
    sent_note_count: usize,
    sent_note_amount: u64,
    counter_value: u64,
    counter_deploy_increment_tx: String,
    last_counter_increment_tx: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let command = env::args().nth(1);

    match command.as_deref() {
        None | Some("run") => {
            let (mut client, keystore) = build_client().await?;
            let artifacts = run_official_activity(&mut client, &keystore).await?;
            write_artifacts(&artifacts)?;

            println!(
                "\nSaved activity artifacts to {}",
                Path::new(ARTIFACTS_PATH).display()
            );
        }
        Some("increment") => {
            let (mut client, _keystore) = build_client().await?;
            let explicit_counter_id = env::args().nth(2);
            let counter_id = match explicit_counter_id {
                Some(bech32) => parse_account_id(&bech32)?,
                None => parse_account_id(&read_artifacts()?.counter_contract_id)?,
            };

            let (counter_value, tx_id) = increment_existing_counter(&mut client, counter_id).await?;
            println!("Counter value is now {counter_value}.");

            if let Ok(mut artifacts) = read_artifacts() {
                artifacts.counter_contract_id = counter_id.to_bech32(NetworkId::Testnet);
                artifacts.counter_value = counter_value;
                artifacts.last_counter_increment_tx = tx_id.to_hex();
                write_artifacts(&artifacts)?;
            }
        }
        Some("help") | Some("--help") | Some("-h") => {
            print_help();
        }
        Some(other) => {
            eprintln!("Unknown command: {other}");
            print_help();
            std::process::exit(2);
        }
    }

    Ok(())
}

async fn build_client() -> Result<(Client<FilesystemKeyStore>, Arc<FilesystemKeyStore>), ClientError>
{
    let endpoint = Endpoint::testnet();
    let timeout_ms = 10_000;
    let rpc_client = Arc::new(GrpcClient::new(&endpoint, timeout_ms));

    let keystore_path = std::path::PathBuf::from("./keystore");
    let keystore = Arc::new(FilesystemKeyStore::new(keystore_path).unwrap());

    let store_path = std::path::PathBuf::from("./store.sqlite3");

    let mut client = ClientBuilder::new()
        .rpc(rpc_client)
        .sqlite_store(store_path)
        .authenticator(keystore.clone())
        .in_debug_mode(true.into())
        .build()
        .await?;

    let sync_summary = client.sync_state().await?;
    println!("Latest block: {}", sync_summary.block_num);

    Ok((client, keystore))
}

async fn run_official_activity(
    client: &mut Client<FilesystemKeyStore>,
    keystore: &Arc<FilesystemKeyStore>,
) -> Result<ActivityArtifacts, Box<dyn Error>> {
    println!("\n[STEP 1] Creating a new account for Alice");
    let alice_account = create_wallet(client, keystore).await?;
    println!(
        "Alice's account ID: {}",
        alice_account.id().to_bech32(NetworkId::Testnet)
    );

    println!("\n[STEP 2] Deploying a new fungible faucet");
    let faucet_account = deploy_faucet(client, keystore).await?;
    println!(
        "Faucet account ID: {}",
        faucet_account.id().to_bech32(NetworkId::Testnet)
    );

    println!("\n[STEP 3] Minting 5 notes of 100 tokens each for Alice");
    let minted_note_count = 5;
    let minted_note_amount = 100_u64;
    mint_notes_to_account(
        client,
        faucet_account.id(),
        alice_account.id(),
        minted_note_count,
        minted_note_amount,
    )
    .await?;

    println!("\n[STEP 4] Alice consumes all minted notes");
    consume_expected_notes(client, alice_account.id(), minted_note_count).await?;

    println!("\n[STEP 5] Alice sends 5 notes of 50 tokens each to 5 different users");
    let sent_note_count = 5;
    let sent_note_amount = 50_u64;
    send_public_p2id_notes(
        client,
        alice_account.id(),
        faucet_account.id(),
        sent_note_count,
        sent_note_amount,
    )
    .await?;

    println!("\n[STEP 6] Deploying and incrementing the official counter contract");
    let (counter_contract_id, counter_value, deploy_tx_id) =
        deploy_and_increment_counter(client).await?;

    let artifacts = ActivityArtifacts {
        network: "testnet".to_string(),
        alice_account_id: alice_account.id().to_bech32(NetworkId::Testnet),
        faucet_account_id: faucet_account.id().to_bech32(NetworkId::Testnet),
        counter_contract_id: counter_contract_id.to_bech32(NetworkId::Testnet),
        minted_note_count,
        minted_note_amount,
        sent_note_count,
        sent_note_amount,
        counter_value,
        counter_deploy_increment_tx: deploy_tx_id.to_hex(),
        last_counter_increment_tx: deploy_tx_id.to_hex(),
    };

    println!(
        "\nAll official tutorial steps completed. Counter ID: {}",
        artifacts.counter_contract_id
    );
    println!(
        "View the counter deploy/increment transaction on MidenScan: https://testnet.midenscan.com/tx/{}",
        artifacts.counter_deploy_increment_tx
    );

    Ok(artifacts)
}

async fn create_wallet(
    client: &mut Client<FilesystemKeyStore>,
    keystore: &Arc<FilesystemKeyStore>,
) -> Result<miden_client::account::Account, Box<dyn Error>> {
    let mut seed = [0_u8; 32];
    client.rng().fill_bytes(&mut seed);

    let key_pair = AuthSecretKey::new_falcon512_rpo();

    let account = AccountBuilder::new(seed)
        .account_type(AccountType::RegularAccountUpdatableCode)
        .storage_mode(AccountStorageMode::Public)
        .with_auth_component(AuthFalcon512Rpo::new(key_pair.public_key().to_commitment()))
        .with_component(BasicWallet)
        .build()?;

    client.add_account(&account, false).await?;
    keystore.add_key(&key_pair)?;

    Ok(account)
}

async fn deploy_faucet(
    client: &mut Client<FilesystemKeyStore>,
    keystore: &Arc<FilesystemKeyStore>,
) -> Result<miden_client::account::Account, Box<dyn Error>> {
    let mut seed = [0_u8; 32];
    client.rng().fill_bytes(&mut seed);

    let key_pair = AuthSecretKey::new_falcon512_rpo();
    let faucet = AccountBuilder::new(seed)
        .account_type(AccountType::FungibleFaucet)
        .storage_mode(AccountStorageMode::Public)
        .with_auth_component(AuthFalcon512Rpo::new(key_pair.public_key().to_commitment()))
        .with_component(
            BasicFungibleFaucet::new(TokenSymbol::new("MID")?, 8, Felt::new(1_000_000))?,
        )
        .build()?;

    client.add_account(&faucet, false).await?;
    keystore.add_key(&key_pair)?;
    client.sync_state().await?;
    sleep(Duration::from_secs(2)).await;

    Ok(faucet)
}

async fn mint_notes_to_account(
    client: &mut Client<FilesystemKeyStore>,
    faucet_id: AccountId,
    recipient_id: AccountId,
    note_count: usize,
    amount: u64,
) -> Result<(), Box<dyn Error>> {
    let asset = miden_client::asset::FungibleAsset::new(faucet_id, amount)?;

    for index in 1..=note_count {
        let request = TransactionRequestBuilder::new()
            .build_mint_fungible_asset(asset, recipient_id, NoteType::Public, client.rng())?;
        let tx_id = client.submit_new_transaction(faucet_id, request).await?;
        println!("Minted note #{index} of {amount} tokens. TX: {}", tx_id.to_hex());
    }

    client.sync_state().await?;
    Ok(())
}

async fn consume_expected_notes(
    client: &mut Client<FilesystemKeyStore>,
    account_id: AccountId,
    expected_count: usize,
) -> Result<(), Box<dyn Error>> {
    loop {
        client.sync_state().await?;

        let consumable_notes = client.get_consumable_notes(Some(account_id)).await?;
        let notes = consumable_notes
            .iter()
            .map(|(note, _)| note.clone().try_into())
            .collect::<Result<Vec<_>, _>>()?;

        if notes.len() == expected_count {
            let request = TransactionRequestBuilder::new().build_consume_notes(notes)?;
            let tx_id = client.submit_new_transaction(account_id, request).await?;
            println!("Consumed {expected_count} notes. TX: {}", tx_id.to_hex());
            wait_for_tx(client, tx_id).await?;
            return Ok(());
        }

        println!(
            "Currently, account has {} consumable notes. Waiting...",
            notes.len()
        );
        sleep(Duration::from_secs(3)).await;
    }
}

async fn send_public_p2id_notes(
    client: &mut Client<FilesystemKeyStore>,
    sender_id: AccountId,
    faucet_id: AccountId,
    note_count: usize,
    amount: u64,
) -> Result<(), Box<dyn Error>> {
    let batch_count = note_count.saturating_sub(1);
    let mut batched_notes = Vec::with_capacity(batch_count);

    for _ in 0..batch_count {
        batched_notes.push(create_dummy_p2id_note(client, sender_id, faucet_id, amount)?);
    }

    let output_notes: Vec<OutputNote> = batched_notes.into_iter().map(OutputNote::Full).collect();
    let request = TransactionRequestBuilder::new()
        .own_output_notes(output_notes)
        .build()?;
    let batch_tx_id = client.submit_new_transaction(sender_id, request).await?;
    println!(
        "Submitted a transaction with {batch_count} P2ID notes. TX: {}",
        batch_tx_id.to_hex()
    );
    wait_for_tx(client, batch_tx_id).await?;

    let final_note = create_dummy_p2id_note(client, sender_id, faucet_id, amount)?;
    let request = TransactionRequestBuilder::new()
        .own_output_notes(vec![OutputNote::Full(final_note)])
        .build()?;
    let single_tx_id = client.submit_new_transaction(sender_id, request).await?;
    println!(
        "Submitted the final single P2ID transaction. TX: {}",
        single_tx_id.to_hex()
    );
    wait_for_tx(client, single_tx_id).await?;

    Ok(())
}

async fn deploy_and_increment_counter(
    client: &mut Client<FilesystemKeyStore>,
) -> Result<(AccountId, u64, TransactionId), Box<dyn Error>> {
    let counter_code = fs::read_to_string(COUNTER_ACCOUNT_CODE_PATH)?;
    let counter_slot_name = StorageSlotName::new(COUNTER_SLOT)?;
    let component_code = CodeBuilder::new()
        .compile_component_code("external_contract::counter_contract", &counter_code)?;

    let counter_component = AccountComponent::new(
        component_code,
        vec![StorageSlot::with_value(
            counter_slot_name.clone(),
            Word::default(),
        )],
    )?
    .with_supports_all_types();

    let mut seed = [0_u8; 32];
    client.rng().fill_bytes(&mut seed);

    let counter_contract = AccountBuilder::new(seed)
        .account_type(AccountType::RegularAccountImmutableCode)
        .storage_mode(AccountStorageMode::Public)
        .with_component(counter_component)
        .with_auth_component(NoAuth)
        .build()?;

    client.add_account(&counter_contract, false).await?;

    let tx_id = submit_counter_increment(client, counter_contract.id(), &counter_code).await?;
    let counter_value = wait_for_counter_value(client, counter_contract.id(), 1).await?;

    println!(
        "Counter contract ID: {}",
        counter_contract.id().to_bech32(NetworkId::Testnet)
    );
    println!("Counter value after deploy flow: {counter_value}");

    Ok((counter_contract.id(), counter_value, tx_id))
}

async fn increment_existing_counter(
    client: &mut Client<FilesystemKeyStore>,
    counter_contract_id: AccountId,
) -> Result<(u64, TransactionId), Box<dyn Error>> {
    client.import_account_by_id(counter_contract_id).await?;
    let current_value = read_counter_value(client, counter_contract_id).await?;

    println!(
        "Current counter value for {} is {}.",
        counter_contract_id.to_bech32(NetworkId::Testnet),
        current_value
    );

    let counter_code = fs::read_to_string(COUNTER_ACCOUNT_CODE_PATH)?;
    let tx_id = submit_counter_increment(client, counter_contract_id, &counter_code).await?;
    let counter_value = wait_for_counter_value(client, counter_contract_id, current_value + 1).await?;

    println!(
        "View transaction on MidenScan: https://testnet.midenscan.com/tx/{}",
        tx_id.to_hex()
    );

    Ok((counter_value, tx_id))
}

async fn submit_counter_increment(
    client: &mut Client<FilesystemKeyStore>,
    counter_contract_id: AccountId,
    counter_code: &str,
) -> Result<TransactionId, Box<dyn Error>> {
    let script_code = fs::read_to_string(COUNTER_SCRIPT_PATH)?;
    let assembler = TransactionKernel::assembler();
    let account_component_lib = create_library(
        assembler,
        "external_contract::counter_contract",
        counter_code,
    )?;

    let tx_script = client
        .code_builder()
        .with_dynamically_linked_library(&account_component_lib)?
        .compile_tx_script(&script_code)?;

    let request = TransactionRequestBuilder::new()
        .custom_script(tx_script)
        .build()?;
    let tx_id = client
        .submit_new_transaction(counter_contract_id, request)
        .await?;

    println!(
        "Submitted counter increment transaction. TX: {}",
        tx_id.to_hex()
    );
    wait_for_tx(client, tx_id).await?;

    Ok(tx_id)
}

async fn read_counter_value(
    client: &mut Client<FilesystemKeyStore>,
    counter_contract_id: AccountId,
) -> Result<u64, Box<dyn Error>> {
    client.sync_state().await?;
    let account_record = client
        .get_account(counter_contract_id)
        .await?
        .ok_or_else(|| "counter contract not found".to_string())?;

    let account = match account_record.account_data() {
        AccountRecordData::Full(account) => account,
        AccountRecordData::Partial(_) => {
            return Err("counter contract is missing full account data".into())
        }
    };
    let counter_slot_name = StorageSlotName::new(COUNTER_SLOT)?;
    let count: Word = account
        .storage()
        .get_item(&counter_slot_name)?
        .into();

    Ok(count.get(3).ok_or_else(|| "counter word missing value".to_string())?.as_int())
}

async fn wait_for_counter_value(
    client: &mut Client<FilesystemKeyStore>,
    counter_contract_id: AccountId,
    expected_minimum: u64,
) -> Result<u64, Box<dyn Error>> {
    for _ in 0..10 {
        let counter_value = read_counter_value(client, counter_contract_id).await?;
        if counter_value >= expected_minimum {
            return Ok(counter_value);
        }

        println!(
            "Counter value is {counter_value}, waiting for at least {expected_minimum}..."
        );
        sleep(Duration::from_secs(3)).await;
    }

    Err(format!(
        "counter value did not reach {expected_minimum} after waiting"
    )
    .into())
}

fn create_dummy_p2id_note(
    client: &mut Client<FilesystemKeyStore>,
    sender_id: AccountId,
    faucet_id: AccountId,
    amount: u64,
) -> Result<miden_client::note::Note, Box<dyn Error>> {
    let mut seed = [0_u8; 15];
    client.rng().fill_bytes(&mut seed);

    let target_account_id = AccountId::dummy(
        seed,
        AccountIdVersion::Version0,
        AccountType::RegularAccountUpdatableCode,
        AccountStorageMode::Public,
    );

    let asset = miden_client::asset::FungibleAsset::new(faucet_id, amount)?;
    let note = create_p2id_note(
        sender_id,
        target_account_id,
        vec![asset.into()],
        NoteType::Public,
        NoteAttachment::default(),
        client.rng(),
    )?;

    Ok(note)
}

async fn wait_for_tx(
    client: &mut Client<FilesystemKeyStore>,
    tx_id: TransactionId,
) -> Result<(), ClientError> {
    loop {
        client.sync_state().await?;

        let txs = client
            .get_transactions(TransactionFilter::Ids(vec![tx_id]))
            .await?;
        let tx_committed = if !txs.is_empty() {
            matches!(txs[0].status, TransactionStatus::Committed { .. })
        } else {
            false
        };

        if tx_committed {
            println!("Committed transaction {}.", tx_id.to_hex());
            break;
        }

        println!("Transaction {} not yet committed. Waiting...", tx_id.to_hex());
        sleep(Duration::from_secs(2)).await;
    }

    Ok(())
}

fn create_library(
    assembler: Assembler,
    library_path: &str,
    source_code: &str,
) -> Result<Library, Box<dyn Error>> {
    let source_manager = Arc::new(DefaultSourceManager::default());
    let module = Module::parser(ModuleKind::Library).parse_str(
        AssemblyPath::new(library_path),
        source_code,
        source_manager.clone(),
    )?;
    let library = assembler.assemble_library([module])?;
    Ok(library)
}

fn parse_account_id(value: &str) -> Result<AccountId, Box<dyn Error>> {
    let (_, account_id) = AccountId::from_bech32(value)?;
    Ok(account_id)
}

fn read_artifacts() -> Result<ActivityArtifacts, Box<dyn Error>> {
    let contents = fs::read_to_string(ARTIFACTS_PATH)?;
    Ok(serde_json::from_str(&contents)?)
}

fn write_artifacts(artifacts: &ActivityArtifacts) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = Path::new(ARTIFACTS_PATH).parent() {
        fs::create_dir_all(parent)?;
    }

    let json = serde_json::to_string_pretty(artifacts)?;
    fs::write(ARTIFACTS_PATH, json)?;
    Ok(())
}

fn print_help() {
    println!("Usage:");
    println!("  cargo run --release");
    println!("  cargo run --release -- run");
    println!("  cargo run --release -- increment");
    println!("  cargo run --release -- increment <mtst1...>");
}
