// Token storage + browser sign-in via the backend approve flow.

const TOKEN_KEY = "aftercode.token";
const STATE_KEY = "aftercode.oauth_state";

export function getToken(): string | null {
  return localStorage.getItem(TOKEN_KEY);
}

export function setToken(token: string): void {
  localStorage.setItem(TOKEN_KEY, token);
}

export function clearToken(): void {
  localStorage.removeItem(TOKEN_KEY);
}

function randomState(): string {
  const a = new Uint8Array(16);
  crypto.getRandomValues(a);
  return Array.from(a, (b) => b.toString(16).padStart(2, "0")).join("");
}

/** Begin sign-in: stash state, send the browser to the approve page. */
export function startSignIn(apiBase: string): void {
  const state = randomState();
  sessionStorage.setItem(STATE_KEY, state);
  const redirect = window.location.origin + window.location.pathname;
  const url = `${apiBase.replace(/\/$/, "")}/cli/authorize?redirect=${encodeURIComponent(
    redirect,
  )}&state=${state}`;
  window.location.href = url;
}

/**
 * On load, if the URL fragment carries `token`+`state` from the approve
 * redirect, validate state, persist the token, and clean the URL.
 * Returns the captured token, or null if there was nothing/invalid.
 */
export function captureTokenFromHash(
  hash: string = window.location.hash,
): string | null {
  if (!hash || !hash.includes("token=")) return null;
  const params = new URLSearchParams(hash.replace(/^#/, ""));
  const token = params.get("token");
  const state = params.get("state");
  const expected = sessionStorage.getItem(STATE_KEY);
  // Clean the URL regardless so a stale hash never lingers.
  const clean = () => {
    sessionStorage.removeItem(STATE_KEY);
    if (typeof window !== "undefined" && window.history?.replaceState) {
      window.history.replaceState(null, "", window.location.pathname + window.location.search);
    }
  };
  if (!token || !state || state !== expected) {
    clean();
    return null;
  }
  setToken(token);
  clean();
  return token;
}
