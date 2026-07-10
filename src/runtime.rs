use crate::amount::Amount;
use crate::engine::CrownEngine;
use crate::error::{CrownError, CrownResult};
use crate::ids::RedemptionId;
use crate::reports::{id_array_json, json_escape, string_array_json, ProtocolReport};
use crate::vault::{RedemptionKind, RedemptionTicket};
use std::env;

pub fn run_from_env() -> CrownResult<()> {
    let mut args = env::args().skip(1);
    match (args.next().as_deref(), args.next().as_deref()) {
        (Some("scenario"), Some("order")) => {
            println!("{}", scenario_order()?);
            Ok(())
        }
        (Some("scenario"), Some("limits")) => {
            println!("{}", scenario_limits()?);
            Ok(())
        }
        (Some("scenario"), Some("cancel")) => {
            println!("{}", scenario_cancel()?);
            Ok(())
        }
        (Some("scenario"), Some("withdrawals")) => {
            println!("{}", scenario_withdrawals()?);
            Ok(())
        }
        (Some("scenario"), Some("report")) => {
            println!("{}", scenario_report()?);
            Ok(())
        }
        (Some("scenario"), Some(other)) => {
            Err(CrownError::Cli(format!("unknown scenario '{other}'")))
        }
        _ => Err(CrownError::Cli(
            "usage: crown-dtl scenario <order|limits|cancel|withdrawals|report>".to_owned(),
        )),
    }
}

pub fn scenario_order() -> CrownResult<String> {
    let mut engine = CrownEngine::fixture();
    let vault = engine.first_vault_id()?;
    let alice = engine.account_id("alice")?;
    let bob = engine.account_id("bob")?;
    let carol = engine.account_id("carol")?;
    let standard = engine.request_redemption(
        alice,
        vault,
        Amount::from(100_u64),
        RedemptionKind::Standard,
    )?;
    let vip =
        engine.request_redemption(bob, vault, Amount::from(100_u64), RedemptionKind::Priority)?;
    let institutional = engine.request_redemption(
        carol,
        vault,
        Amount::from(100_u64),
        RedemptionKind::Priority,
    )?;
    let queue_order = engine.queue_order(vault);
    let processed = engine.process_vault_queue(vault, 3)?;
    let processed_ids = processed
        .iter()
        .map(|ticket| ticket.id())
        .collect::<Vec<_>>();
    let processed_labels = ticket_labels(&engine, &processed)?;
    let processed_lanes = processed
        .iter()
        .map(|ticket| ticket.kind().label().to_owned())
        .collect::<Vec<_>>();
    Ok(format!(
        "{{\"scenario\":\"order\",\"standard\":\"{}\",\"vip\":\"{}\",\"institutional\":\"{}\",\"queueOrder\":{},\"processed\":{},\"processedLabels\":{},\"processedLanes\":{}}}",
        standard.redemption_id,
        vip.redemption_id,
        institutional.redemption_id,
        id_array_json(&queue_order),
        id_array_json(&processed_ids),
        string_array_json(&processed_labels),
        string_array_json(&processed_lanes)
    ))
}

pub fn scenario_limits() -> CrownResult<String> {
    let mut engine = CrownEngine::fixture();
    let vault = engine.first_vault_id()?;
    let bob = engine.account_id("bob")?;
    let first =
        engine.request_redemption(bob, vault, Amount::from(900_u64), RedemptionKind::Priority)?;
    let rejected_code = match engine.request_redemption(
        bob,
        vault,
        Amount::from(200_u64),
        RedemptionKind::Priority,
    ) {
        Ok(_) => "accepted".to_owned(),
        Err(err) => err.code().to_owned(),
    };
    engine.cancel_redemption(first.redemption_id)?;
    let active_after_cancel = engine.daily_active(bob, vault, engine.day())?;
    let capacity_after_cancel = engine.priority_available(vault, engine.day())?;
    let second =
        engine.request_redemption(bob, vault, Amount::from(900_u64), RedemptionKind::Priority)?;
    Ok(format!(
        "{{\"scenario\":\"limits\",\"first\":\"{}\",\"second\":\"{}\",\"rejectedCode\":\"{}\",\"activeAfterCancel\":{},\"capacityAfterCancel\":{}}}",
        first.redemption_id,
        second.redemption_id,
        json_escape(&rejected_code),
        active_after_cancel.raw(),
        capacity_after_cancel.raw()
    ))
}

pub fn scenario_cancel() -> CrownResult<String> {
    let mut engine = CrownEngine::fixture();
    let vault = engine.first_vault_id()?;
    let bob = engine.account_id("bob")?;
    let first =
        engine.request_redemption(bob, vault, Amount::from(500_u64), RedemptionKind::Priority)?;
    let cancelled = engine.cancel_redemption(first.redemption_id)?;
    let withdraw_cancelled_code = match engine.withdraw_ticket(first.redemption_id) {
        Ok(_) => "accepted".to_owned(),
        Err(err) => err.code().to_owned(),
    };
    let shares_after_cancel = engine
        .accounts()
        .get(bob)?
        .portfolio()
        .share_balance(vault)
        .raw();
    let capacity_after_cancel = engine.priority_available(vault, engine.day())?;
    let second =
        engine.request_redemption(bob, vault, Amount::from(500_u64), RedemptionKind::Priority)?;
    let processed = engine.process_vault_queue(vault, 1)?;
    engine.advance_days(1)?;
    let withdrawal = engine.withdraw_ticket(second.redemption_id)?;
    Ok(format!(
        "{{\"scenario\":\"cancel\",\"cancelled\":\"{}\",\"cancelledStatus\":\"{}\",\"withdrawCancelledCode\":\"{}\",\"replacement\":\"{}\",\"processed\":{},\"sharesAfterCancel\":{},\"capacityAfterCancel\":{},\"withdrawn\":{}}}",
        cancelled.id(),
        cancelled.status().label(),
        json_escape(&withdraw_cancelled_code),
        second.redemption_id,
        id_array_json(&processed.iter().map(|ticket| ticket.id()).collect::<Vec<_>>()),
        shares_after_cancel,
        capacity_after_cancel.raw(),
        withdrawal.amount.raw()
    ))
}

pub fn scenario_withdrawals() -> CrownResult<String> {
    let mut engine = CrownEngine::fixture();
    let vault = engine.first_vault_id()?;
    let bob = engine.account_id("bob")?;
    let alice = engine.account_id("alice")?;
    let priority =
        engine.request_redemption(bob, vault, Amount::from(300_u64), RedemptionKind::Priority)?;
    let standard = engine.request_redemption(
        alice,
        vault,
        Amount::from(200_u64),
        RedemptionKind::Standard,
    )?;
    let processed = engine.process_vault_queue(vault, 2)?;
    let early = engine.withdraw_available(bob, vault)?;
    engine.advance_days(1)?;
    let vip_withdrawal = engine.withdraw_available(bob, vault)?;
    engine.advance_days(2)?;
    let standard_withdrawal = engine.withdraw_available(alice, vault)?;
    let report = ProtocolReport::capture(&engine)?;
    Ok(format!(
        "{{\"scenario\":\"withdrawals\",\"priority\":\"{}\",\"standard\":\"{}\",\"processed\":{},\"early\":{},\"vipWithdrawal\":{},\"standardWithdrawal\":{},\"report\":{}}}",
        priority.redemption_id,
        standard.redemption_id,
        id_array_json(&processed.iter().map(|ticket| ticket.id()).collect::<Vec<_>>()),
        early.amount.raw(),
        vip_withdrawal.amount.raw(),
        standard_withdrawal.amount.raw(),
        report.to_json()
    ))
}

pub fn scenario_report() -> CrownResult<String> {
    let engine = CrownEngine::fixture();
    ProtocolReport::capture(&engine).map(|report| report.to_json())
}

fn ticket_labels(engine: &CrownEngine, tickets: &[RedemptionTicket]) -> CrownResult<Vec<String>> {
    let mut labels = Vec::new();
    for ticket in tickets {
        let account = engine.accounts().get(ticket.account())?;
        labels.push(account.label().to_owned());
    }
    Ok(labels)
}

#[allow(dead_code)]
fn ids_as_strings(ids: &[RedemptionId]) -> Vec<String> {
    ids.iter().map(ToString::to_string).collect()
}
