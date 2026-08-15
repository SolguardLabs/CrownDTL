const BPS = 10_000n;
const IDENTIFIER = /^[a-z0-9][a-z0-9:_-]{2,127}$/;
const IDEMPOTENCY_KEY = /^[A-Za-z0-9._:-]{16,128}$/;

function requireBigInt(name, value) {
  if (typeof value !== "bigint" || value < 0n) {
    throw new TypeError(`${name} must be a non-negative bigint`);
  }
  return value;
}

function requireIdentifier(name, value) {
  if (typeof value !== "string" || !IDENTIFIER.test(value)) {
    throw new TypeError(`${name} is not normalized`);
  }
  return value;
}

export function mulDivFloor(value, multiplier, denominator) {
  requireBigInt("value", value);
  requireBigInt("multiplier", multiplier);
  requireBigInt("denominator", denominator);
  if (denominator === 0n) throw new RangeError("denominator must be positive");
  return (value * multiplier) / denominator;
}

export function mulDivCeil(value, multiplier, denominator) {
  requireBigInt("value", value);
  requireBigInt("multiplier", multiplier);
  requireBigInt("denominator", denominator);
  if (denominator === 0n) throw new RangeError("denominator must be positive");
  if (value === 0n || multiplier === 0n) return 0n;
  return (value * multiplier + denominator - 1n) / denominator;
}

export function evaluateCapital(input) {
  const vault = requireIdentifier("vault", input.vault);
  const reserveAssets = requireBigInt("reserveAssets", input.reserveAssets);
  const liquidAssets = requireBigInt("liquidAssets", input.liquidAssets);
  const totalShares = requireBigInt("totalShares", input.totalShares);
  const openClaims = requireBigInt("openClaims", input.openClaims);
  const priorityCapacity = requireBigInt("priorityCapacity", input.priorityCapacity);
  const reserveHaircutBps = requireBigInt("reserveHaircutBps", input.reserveHaircutBps);
  const claimShockBps = requireBigInt("claimShockBps", input.claimShockBps);
  const operationalBufferBps = requireBigInt("operationalBufferBps", input.operationalBufferBps);

  if (liquidAssets > reserveAssets) throw new RangeError("liquid assets exceed reserve");
  if (totalShares === 0n) throw new RangeError("total shares must be positive");
  if (reserveHaircutBps > BPS) throw new RangeError("reserve haircut exceeds 10000 bps");

  const effectiveReserve = mulDivFloor(reserveAssets, BPS - reserveHaircutBps, BPS);
  const stressedClaims = mulDivCeil(openClaims, BPS + claimShockBps, BPS);
  const operationalBuffer = mulDivCeil(totalShares, operationalBufferBps, BPS);
  const requiredReserve = stressedClaims + operationalBuffer;
  const coverageBps =
    requiredReserve === 0n ? BPS : mulDivFloor(effectiveReserve, BPS, requiredReserve);
  const liquidityBps = reserveAssets === 0n ? 0n : mulDivFloor(liquidAssets, BPS, reserveAssets);
  const claimShareBps = totalShares === 0n ? 0n : mulDivCeil(openClaims, BPS, totalShares);
  const priorityUtilizationBps =
    priorityCapacity === 0n ? 0n : mulDivCeil(openClaims, BPS, priorityCapacity);
  const availableReserve = effectiveReserve < liquidAssets ? effectiveReserve : liquidAssets;
  const reserveShortfall =
    requiredReserve > availableReserve ? requiredReserve - availableReserve : 0n;
  const capacityHeadroom =
    availableReserve > requiredReserve ? availableReserve - requiredReserve : 0n;
  const availablePriorityCapacity =
    priorityCapacity < capacityHeadroom ? priorityCapacity : capacityHeadroom;

  return Object.freeze({
    vault,
    effectiveReserve,
    stressedClaims,
    operationalBuffer,
    requiredReserve,
    reserveShortfall,
    coverageBps,
    liquidityBps,
    claimShareBps,
    priorityUtilizationBps,
    availablePriorityCapacity,
    compliant: reserveShortfall === 0n,
  });
}

export function canonicalJson(value) {
  if (typeof value === "bigint") return value.toString();
  if (value === null || typeof value === "boolean" || typeof value === "number") {
    if (typeof value === "number" && !Number.isSafeInteger(value)) {
      throw new TypeError("numbers must be safe integers");
    }
    return JSON.stringify(value);
  }
  if (typeof value === "string") return JSON.stringify(value);
  if (Array.isArray(value)) return `[${value.map(canonicalJson).join(",")}]`;
  if (typeof value === "object") {
    const entries = Object.keys(value)
      .sort()
      .filter((key) => value[key] !== undefined)
      .map((key) => `${JSON.stringify(key)}:${canonicalJson(value[key])}`);
    return `{${entries.join(",")}}`;
  }
  throw new TypeError("value cannot be represented canonically");
}

export class CrownClient {
  #baseUrl;
  #fetch;
  #timeoutMs;

  constructor({ baseUrl, fetchImpl = globalThis.fetch, timeoutMs = 8_000 }) {
    const parsed = new URL(baseUrl);
    if (parsed.protocol !== "https:") throw new TypeError("baseUrl must use HTTPS");
    if (parsed.username || parsed.password || parsed.search || parsed.hash) {
      throw new TypeError("baseUrl must not contain credentials, query, or fragment");
    }
    if (typeof fetchImpl !== "function") throw new TypeError("fetch implementation is required");
    if (!Number.isSafeInteger(timeoutMs) || timeoutMs < 100 || timeoutMs > 60_000) {
      throw new RangeError("timeoutMs is outside the supported range");
    }
    this.#baseUrl = new URL(parsed.pathname.endsWith("/") ? parsed : `${parsed}/`);
    this.#fetch = fetchImpl;
    this.#timeoutMs = timeoutMs;
  }

  async protocolState({ signal } = {}) {
    return this.#request("v1/state", { method: "GET", signal });
  }

  async submitRedemption(input, { idempotencyKey, signal } = {}) {
    if (!IDEMPOTENCY_KEY.test(idempotencyKey ?? "")) {
      throw new TypeError("a normalized idempotency key is required");
    }
    const kind = input.kind;
    if (kind !== "standard" && kind !== "priority") {
      throw new TypeError("kind must be standard or priority");
    }
    const body = canonicalJson({
      accountId: requireIdentifier("accountId", input.accountId),
      kind,
      shares: requireBigInt("shares", input.shares),
      vaultId: requireIdentifier("vaultId", input.vaultId),
      windowId: requireIdentifier("windowId", input.windowId),
    });
    return this.#request("v1/redemptions", {
      method: "POST",
      body,
      headers: { "Idempotency-Key": idempotencyKey },
      signal,
    });
  }

  async cancelRedemption(redemptionId, { idempotencyKey, signal } = {}) {
    if (!IDEMPOTENCY_KEY.test(idempotencyKey ?? "")) {
      throw new TypeError("a normalized idempotency key is required");
    }
    const id = requireIdentifier("redemptionId", redemptionId);
    return this.#request(`v1/redemptions/${encodeURIComponent(id)}/cancellation`, {
      method: "POST",
      body: "{}",
      headers: { "Idempotency-Key": idempotencyKey },
      signal,
    });
  }

  async withdrawAvailable({ accountId, vaultId }, { idempotencyKey, signal } = {}) {
    if (!IDEMPOTENCY_KEY.test(idempotencyKey ?? "")) {
      throw new TypeError("a normalized idempotency key is required");
    }
    const body = canonicalJson({
      accountId: requireIdentifier("accountId", accountId),
      vaultId: requireIdentifier("vaultId", vaultId),
    });
    return this.#request("v1/withdrawals", {
      method: "POST",
      body,
      headers: { "Idempotency-Key": idempotencyKey },
      signal,
    });
  }

  async #request(path, options) {
    const timeout = AbortSignal.timeout(this.#timeoutMs);
    const signal = options.signal ? AbortSignal.any([options.signal, timeout]) : timeout;
    const response = await this.#fetch(new URL(path, this.#baseUrl), {
      ...options,
      signal,
      headers: {
        Accept: "application/json",
        "Content-Type": "application/json",
        ...options.headers,
      },
      redirect: "error",
    });
    const contentType = response.headers.get("content-type") ?? "";
    if (!contentType.toLowerCase().startsWith("application/json")) {
      throw new Error(`unexpected response type (${response.status})`);
    }
    const payload = await response.json();
    if (!response.ok) {
      const error = new Error(payload.message ?? `request failed (${response.status})`);
      error.status = response.status;
      error.code = payload.code ?? "CROWN_REQUEST_FAILED";
      throw error;
    }
    return payload;
  }
}
