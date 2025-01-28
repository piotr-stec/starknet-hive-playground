use openrpc_testgen::utils::{
    get_deployed_contract_address::get_contract_address,
    starknet_hive::StarkneHive,
    v7::{
        accounts::{
            account::{Account, ConnectedAccount},
            call::Call,
        },
        contract::factory::ContractFactory,
        endpoints::{
            declare_contract::get_compiled_contract,
            errors::OpenRpcTestGenError,
            utils::{get_selector_from_name, wait_for_sent_transaction},
        },
        providers::provider::Provider,
    },
};
use rand::{rngs::StdRng, RngCore, SeedableRng};
use starknet_types_core::felt::Felt;
use starknet_types_rpc::{BlockId, BlockTag, FunctionCall};
use std::{path::PathBuf, str::FromStr};
use url::Url;

#[tokio::main]
async fn main() -> Result<(), OpenRpcTestGenError> {
    // StarknetHive initialization
    let hive = StarkneHive::new(
        Url::parse("http://localhost:5050").unwrap(),
        Felt::from_hex_unchecked(
            "0x064b48806902a367c8598f4f95c305e8c1a1acba5f082d294a43793113115691",
        ),
        Felt::from_hex_unchecked(
            "0x0000000000000000000000000000000071d7bb07b9a64f6f78ac4c816aff4da9",
        ),
        Felt::from_hex_unchecked(
            "0x061dac032f228abef9c6626f995015233097ae253a7f72d68552db02f2971b8f",
        ),
    )
    .await?;

    // ----- DECLARE V3 -----

    let (flattened_sierra_class, compiled_class_hash) = get_compiled_contract(
        PathBuf::from_str("target/dev/contracts_HelloStarknet.contract_class.json").unwrap(),
        PathBuf::from_str("target/dev/contracts_HelloStarknet.compiled_contract_class.json")
            .unwrap(),
    )
    .await?;

    let result = hive
        .declare_v3(flattened_sierra_class, compiled_class_hash)
        .send()
        .await?;

    wait_for_sent_transaction(result.transaction_hash, &hive.account).await?;

    println!("Result: {result:#?}");

    // ----- DEPLOY V3 -----

    let factory = ContractFactory::new(result.class_hash, &hive.account);

    let mut salt_buffer = [0u8; 32];
    let mut rng = StdRng::from_entropy();
    rng.fill_bytes(&mut salt_buffer[1..]);

    let deploy_result = factory
        .deploy_v3(vec![], Felt::from_bytes_be(&salt_buffer), true)
        .send()
        .await?;

    wait_for_sent_transaction(deploy_result.transaction_hash, &hive.account).await?;

    println!("Result: {:?}", deploy_result);

    // ----- INVOKE V3 -----

    // get contract address of deployed contract
    let deployed_contract_address =
        get_contract_address(hive.provider(), deploy_result.transaction_hash).await?;

    let amount_to_increase = Felt::from_hex_unchecked("0x123");
    let increase_balance_call = Call {
        to: deployed_contract_address,
        selector: get_selector_from_name("increase_balance")?,
        calldata: vec![amount_to_increase],
    };

    // Invoke with normal signature
    let invoke_result = hive
        .execute_v3(vec![increase_balance_call.clone()])
        .send()
        .await?;

    wait_for_sent_transaction(invoke_result.transaction_hash, &hive.account).await?;

    let binding = hive
        .provider()
        .call(
            FunctionCall {
                calldata: vec![],
                contract_address: deployed_contract_address,
                entry_point_selector: get_selector_from_name("get_balance")?,
            },
            BlockId::Tag(BlockTag::Pending),
        )
        .await?;

    let balance = binding.first().ok_or(OpenRpcTestGenError::Other(
        "Empty initial contract balance".to_string(),
    ))?;

    println!("balance {:?}", balance);

    // Invoke with custom signature
    let signature = vec![Felt::ONE, Felt::TWO];

    let invoke_result_custom_signature = hive
        .execute_v3(vec![increase_balance_call])
        .send_with_custom_signature(signature)
        .await?;

    wait_for_sent_transaction(
        invoke_result_custom_signature.transaction_hash,
        &hive.account,
    )
    .await?;

    Ok(())
}
