import test from "node:test";
import assert from "node:assert/strict";
import { runScenario } from "../helpers/runScenario.js";

test("withdrawals respect unlock windows and settle account balances", () => {
  const scenario = runScenario("withdrawals");

  assert.equal(scenario.scenario, "withdrawals");
  assert.deepEqual(scenario.processed, [scenario.priority, scenario.standard]);
  assert.equal(scenario.early, 0);
  assert.equal(scenario.vipWithdrawal, 300);
  assert.equal(scenario.standardWithdrawal, 207);

  const vault = scenario.report.vaults[0];
  assert.equal(vault.queueDepth, 0);
  assert.equal(vault.openClaims, 0);
  assert.equal(vault.tickets.withdrawn, 2);
});
