import test from "node:test";
import assert from "node:assert/strict";
import { runScenario } from "../helpers/runScenario.js";

test("priority queue processes institutional, vip, then standard tickets", () => {
  const scenario = runScenario("order");

  assert.equal(scenario.scenario, "order");
  assert.deepEqual(scenario.queueOrder, [
    scenario.institutional,
    scenario.vip,
    scenario.standard,
  ]);
  assert.deepEqual(scenario.processed, scenario.queueOrder);
  assert.deepEqual(scenario.processedLabels, ["carol", "bob", "alice"]);
  assert.deepEqual(scenario.processedLanes, ["priority", "priority", "standard"]);
});
