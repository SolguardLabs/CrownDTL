use crown_dtl::{Amount, RedemptionKind};

#[test]
fn standard_redemption_schedules_and_withdraws_after_unlock() {
    let mut engine = crown_dtl::fixture_engine();
    let vault = engine.first_vault_id().unwrap();
    let alice = engine.account_id("alice").unwrap();

    let request = engine
        .request_redemption(
            alice,
            vault,
            Amount::from(200_u64),
            RedemptionKind::Standard,
        )
        .unwrap();
    let processed = engine.process_vault_queue(vault, 1).unwrap();
    assert_eq!(processed[0].id(), request.redemption_id);

    let early = engine.withdraw_available(alice, vault).unwrap();
    assert_eq!(early.amount.raw(), 0);

    engine.advance_days(3).unwrap();
    let withdrawal = engine.withdraw_available(alice, vault).unwrap();
    assert_eq!(withdrawal.amount.raw(), 200);
}

#[test]
fn cancellation_reopens_daily_headroom() {
    let mut engine = crown_dtl::fixture_engine();
    let vault = engine.first_vault_id().unwrap();
    let alice = engine.account_id("alice").unwrap();

    let first = engine
        .request_redemption(
            alice,
            vault,
            Amount::from(700_u64),
            RedemptionKind::Standard,
        )
        .unwrap();
    engine.cancel_redemption(first.redemption_id).unwrap();
    assert_eq!(
        engine
            .daily_active(alice, vault, engine.day())
            .unwrap()
            .raw(),
        0
    );

    let second = engine.request_redemption(
        alice,
        vault,
        Amount::from(700_u64),
        RedemptionKind::Standard,
    );
    assert!(second.is_ok());
}

#[test]
fn priority_capacity_reopens_after_cancellation() {
    let mut engine = crown_dtl::fixture_engine();
    let vault = engine.first_vault_id().unwrap();
    let bob = engine.account_id("bob").unwrap();

    let first = engine
        .request_redemption(bob, vault, Amount::from(900_u64), RedemptionKind::Priority)
        .unwrap();
    assert_eq!(
        engine
            .priority_available(vault, engine.day())
            .unwrap()
            .raw(),
        600
    );
    engine.cancel_redemption(first.redemption_id).unwrap();
    assert_eq!(
        engine
            .priority_available(vault, engine.day())
            .unwrap()
            .raw(),
        1500
    );
}
