use anchorkit::contract::{AnchorKitContract, AnchorKitContractClient};
use soroban_sdk::{
    testutils::{Address as _, Ledger, LedgerInfo},
    Address, Bytes, Env, String,
};

fn set_ledger(env: &Env, timestamp: u64) {
    env.ledger().set(LedgerInfo {
        timestamp,
        protocol_version: 21,
        sequence_number: 0,
        network_id: Default::default(),
        base_reserve: 0,
        min_persistent_entry_ttl: 4096,
        min_temp_entry_ttl: 16,
        max_entry_ttl: 6312000,
    });
}

fn setup(timestamp: u64) -> (Env, AnchorKitContractClient<'static>, Address) {
    let env = Env::default();
    env.mock_all_auths();
    set_ledger(&env, timestamp);
    let admin = Address::generate(&env);
    let client =
        AnchorKitContractClient::new(&env, &env.register_contract(None, AnchorKitContract));
    client.initialize(&admin, &100_u64, &None);
    (env, client, admin)
}

#[test]
fn zero_stake_reward_gap_is_not_paid_to_late_staker() {
    let (env, client, _admin) = setup(1_000);
    let farmer = Address::generate(&env);

    client.configure_yield_farm(&10_u128, &1_100_u64);
    set_ledger(&env, 1_050);
    client.stake_yield_farm(&farmer, &100_u128);

    set_ledger(&env, 1_060);
    let claim = client.claim_yield_rewards(&farmer);

    assert_eq!(claim.amount, 100_u128);
}

#[test]
fn rewards_stop_at_period_finish() {
    let (env, client, _admin) = setup(2_000);
    let farmer = Address::generate(&env);

    client.configure_yield_farm(&10_u128, &2_010_u64);
    client.stake_yield_farm(&farmer, &100_u128);

    set_ledger(&env, 2_020);
    let claim = client.claim_yield_rewards(&farmer);

    assert_eq!(claim.amount, 100_u128);
}

#[test]
fn withdrawing_more_than_staked_is_rejected() {
    let (env, client, _admin) = setup(3_000);
    let farmer = Address::generate(&env);

    client.configure_yield_farm(&1_u128, &3_100_u64);
    client.stake_yield_farm(&farmer, &25_u128);
    assert!(client.try_withdraw_yield_farm(&farmer, &26_u128).is_err());
}

#[test]
fn yield_farm_requires_future_period_finish() {
    let (_env, client, _admin) = setup(3_500);

    assert!(client
        .try_configure_yield_farm(&1_u128, &3_500_u64)
        .is_err());
}

#[test]
fn allowed_event_hub_publisher_can_publish_and_retrieve() {
    let (env, client, _admin) = setup(4_000);
    let publisher = Address::generate(&env);
    let topic = String::from_str(&env, "attestation");
    let data = Bytes::from_slice(&env, b"ready");

    client.set_event_hub_publisher(&publisher, &true);
    let message = client.publish_hub_event(&publisher, &topic, &data);
    let stored = client.get_hub_event(&message.event_id).unwrap();

    assert_eq!(message.event_id, 0);
    assert_eq!(stored.publisher, publisher);
    assert_eq!(stored.topic, topic);
    assert_eq!(stored.data, data);
}

#[test]
fn event_hub_rejects_unapproved_publisher() {
    let (env, client, _admin) = setup(5_000);
    let publisher = Address::generate(&env);

    let result = client.try_publish_hub_event(
        &publisher,
        &String::from_str(&env, "attestation"),
        &Bytes::from_slice(&env, b"blocked"),
    );
    assert!(result.is_err());
}
