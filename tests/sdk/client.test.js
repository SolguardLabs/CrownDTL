import test from "node:test";
import assert from "node:assert/strict";
import { CrownClient } from "../../sdk/crownClient.js";

function response(payload, status = 200) {
  return new Response(JSON.stringify(payload), {
    status,
    headers: { "content-type": "application/json; charset=utf-8" },
  });
}

test("client submits canonical redemptions with idempotency", async () => {
  let observed;
  const client = new CrownClient({
    baseUrl: "https://api.crowndtl.example/settlement",
    fetchImpl: async (url, options) => {
      observed = { url: url.toString(), options };
      return response({ redemptionId: "redemption:1042", status: "queued" }, 202);
    },
  });
  const result = await client.submitRedemption(
    {
      accountId: "account:treasury",
      vaultId: "vault:senior",
      windowId: "window:eu-1",
      shares: 9_007_199_254_740_993n,
      kind: "priority",
    },
    { idempotencyKey: "redemption-20260815-0001" },
  );
  assert.equal(result.status, "queued");
  assert.equal(observed.url, "https://api.crowndtl.example/settlement/v1/redemptions");
  assert.equal(observed.options.headers["Idempotency-Key"], "redemption-20260815-0001");
  assert.match(observed.options.body, /9007199254740993/);
});

test("client rejects insecure endpoints and malformed keys", async () => {
  assert.throws(() => new CrownClient({ baseUrl: "http://api.example" }), /HTTPS/);
  const client = new CrownClient({
    baseUrl: "https://api.example",
    fetchImpl: async () => response({}),
  });
  await assert.rejects(
    client.withdrawAvailable(
      { accountId: "account:one", vaultId: "vault:one" },
      { idempotencyKey: "short" },
    ),
    /idempotency/,
  );
});

test("client returns structured remote errors", async () => {
  const client = new CrownClient({
    baseUrl: "https://api.example",
    fetchImpl: async () =>
      response({ code: "CAPACITY_EXHAUSTED", message: "capacity unavailable" }, 409),
  });
  await assert.rejects(
    client.cancelRedemption("redemption:1042", { idempotencyKey: "cancel-20260815-0001" }),
    (error) => error.status === 409 && error.code === "CAPACITY_EXHAUSTED",
  );
});
