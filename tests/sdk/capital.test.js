import test from "node:test";
import assert from "node:assert/strict";
import { canonicalJson, evaluateCapital, mulDivCeil, mulDivFloor } from "../../sdk/crownClient.js";

test("capital evaluation mirrors conservative reserve rules", () => {
  const result = evaluateCapital({
    vault: "vault:senior",
    reserveAssets: 1_000_000n,
    liquidAssets: 800_000n,
    totalShares: 900_000n,
    openClaims: 300_000n,
    priorityCapacity: 500_000n,
    reserveHaircutBps: 500n,
    claimShockBps: 2_000n,
    operationalBufferBps: 800n,
  });
  assert.equal(result.effectiveReserve, 950_000n);
  assert.equal(result.requiredReserve, 432_000n);
  assert.equal(result.reserveShortfall, 0n);
  assert.equal(result.availablePriorityCapacity, 368_000n);
  assert.equal(result.compliant, true);
});

test("integer helpers round in the declared direction", () => {
  assert.equal(mulDivFloor(10n, 1n, 3n), 3n);
  assert.equal(mulDivCeil(10n, 1n, 3n), 4n);
  assert.throws(() => mulDivFloor(1n, 1n, 0n), /positive/);
});

test("capital validation rejects inconsistent liquidity", () => {
  assert.throws(
    () =>
      evaluateCapital({
        vault: "vault:senior",
        reserveAssets: 10n,
        liquidAssets: 11n,
        totalShares: 10n,
        openClaims: 1n,
        priorityCapacity: 5n,
        reserveHaircutBps: 0n,
        claimShockBps: 0n,
        operationalBufferBps: 0n,
      }),
    /liquid assets exceed reserve/,
  );
});

test("capital evaluation requires liquid coverage", () => {
  const result = evaluateCapital({
    vault: "vault:senior",
    reserveAssets: 1_000_000n,
    liquidAssets: 400_000n,
    totalShares: 900_000n,
    openClaims: 300_000n,
    priorityCapacity: 500_000n,
    reserveHaircutBps: 500n,
    claimShockBps: 2_000n,
    operationalBufferBps: 800n,
  });
  assert.equal(result.reserveShortfall, 32_000n);
  assert.equal(result.compliant, false);
});

test("canonical JSON sorts keys and preserves integer precision", () => {
  assert.equal(
    canonicalJson({ z: 9_007_199_254_740_993n, a: "crown" }),
    '{"a":"crown","z":9007199254740993}',
  );
});
