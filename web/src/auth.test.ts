import { describe, it, expect, beforeEach } from "vitest";
import { captureTokenFromHash, getToken, clearToken } from "./auth";

class MemStorage {
  private m = new Map<string, string>();
  getItem(k: string) {
    return this.m.has(k) ? this.m.get(k)! : null;
  }
  setItem(k: string, v: string) {
    this.m.set(k, String(v));
  }
  removeItem(k: string) {
    this.m.delete(k);
  }
  clear() {
    this.m.clear();
  }
}

beforeEach(() => {
  (globalThis as unknown as { localStorage: Storage }).localStorage = new MemStorage() as unknown as Storage;
  (globalThis as unknown as { sessionStorage: Storage }).sessionStorage = new MemStorage() as unknown as Storage;
});

describe("captureTokenFromHash", () => {
  it("saves token when state matches", () => {
    sessionStorage.setItem("aftercode.oauth_state", "S1");
    const tok = captureTokenFromHash("#token=ak_abc&state=S1");
    expect(tok).toBe("ak_abc");
    expect(getToken()).toBe("ak_abc");
  });

  it("rejects token when state mismatches", () => {
    sessionStorage.setItem("aftercode.oauth_state", "GOOD");
    const tok = captureTokenFromHash("#token=ak_abc&state=BAD");
    expect(tok).toBeNull();
    expect(getToken()).toBeNull();
  });

  it("returns null with no hash", () => {
    expect(captureTokenFromHash("")).toBeNull();
  });

  it("clearToken removes it", () => {
    sessionStorage.setItem("aftercode.oauth_state", "S");
    captureTokenFromHash("#token=ak_x&state=S");
    clearToken();
    expect(getToken()).toBeNull();
  });
});
