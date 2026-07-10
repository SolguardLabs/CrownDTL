import test from "node:test";
import assert from "node:assert/strict";
import { runScenario } from "../helpers/runScenario.js";

test("cancelled priority tickets restore shares and cannot be withdrawn by id", () => {
  const scenario = runScenario("cancel");

  assert.equal(scenario.scenario, "cancel");
  assert.equal(scenario.cancelledStatus, "cancelled");
  assert.equal(scenario.withdrawCancelledCode, "invalid_status");
  assert.equal(scenario.sharesAfterCancel, 2000);
  assert.equal(scenario.capacityAfterCancel, 1500);
  assert.deepEqual(scenario.processed, [scenario.replacement]);
  assert.equal(scenario.withdrawn, 500);
});
