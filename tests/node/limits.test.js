import test from "node:test";
import assert from "node:assert/strict";
import { runScenario } from "../helpers/runScenario.js";

test("daily and priority capacity limits are reusable after cancellation", () => {
  const scenario = runScenario("limits");

  assert.equal(scenario.scenario, "limits");
  assert.equal(scenario.rejectedCode, "limit_exceeded");
  assert.equal(scenario.activeAfterCancel, 0);
  assert.equal(scenario.capacityAfterCancel, 1500);
  assert.notEqual(scenario.first, scenario.second);
});
