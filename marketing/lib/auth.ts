"use client";

import { apiFetch } from "@/lib/api";

/**
 * Sign-In-With-Solana session for the marketing site.
 *
 * Every RP write (claim, support, spend) is bound server-side to the wallet
 * that signed a nonce. The bearer token is kept in localStorage; only its
 * SHA-256 is stored on the server, and it expires after 7 days.
 */

const STORAGE_KEY = "auctioning.session";
const ALPHABET = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

type PhantomProvider = {
  isPhantom?: boolean;
  publicKey?: { toString: () => string } | null;
  connect: () => Promise<{ publicKey: { toString: () => string } }>;
  signMessage: (message: Uint8Array, display?: "utf8" | "hex") => Promise<{ signature: Uint8Array }>;
};

export function phantom(): PhantomProvider | null {
  if (typeof window === "undefined") return null;
  const w = window as unknown as {
    solana?: PhantomProvider;
    phantom?: { solana?: PhantomProvider };
  };
  if (w.phantom?.solana?.isPhantom) return w.phantom.solana;
  if (w.solana?.isPhantom) return w.solana;
  return null;
}

export function base58(bytes: Uint8Array): string {
  const digits = [0];
  for (const byte of bytes) {
    let carry = byte;
    for (let j = 0; j < digits.length; j++) {
      carry += digits[j] << 8;
      digits[j] = carry % 58;
      carry = (carry / 58) | 0;
    }
    while (carry > 0) {
      digits.push(carry % 58);
      carry = (carry / 58) | 0;
    }
  }
  let out = "";
  for (let k = 0; k < bytes.length && bytes[k] === 0; k++) out += ALPHABET[0];
  for (let d = digits.length - 1; d >= 0; d--) out += ALPHABET[digits[d]];
  return out;
}

export function sessionToken(): string | null {
  if (typeof window === "undefined") return null;
  try {
    return window.localStorage.getItem(STORAGE_KEY);
  } catch {
    return null;
  }
}

function storeToken(token: string | null) {
  try {
    if (token) window.localStorage.setItem(STORAGE_KEY, token);
    else window.localStorage.removeItem(STORAGE_KEY);
  } catch {
    /* private mode */
  }
}

export function authHeaders(): Record<string, string> {
  const token = sessionToken();
  return token ? { authorization: `Bearer ${token}` } : {};
}

type Nonce = { nonce: string; message: string };
type Grant = { token: string; wallet: string; expires_at: string };

/** nonce → Phantom signMessage → verify → bearer token. */
export async function signIn(wallet: string): Promise<{ ok: true } | { ok: false; error: string }> {
  const p = phantom();
  if (!p) return { ok: false, error: "Phantom not detected" };
  const nonce = await apiFetch<Nonce>(`/v1/auth/nonce?wallet=${encodeURIComponent(wallet)}`);
  if (!nonce.ok) return { ok: false, error: nonce.error };
  let signature: string;
  try {
    const signed = await p.signMessage(new TextEncoder().encode(nonce.data.message), "utf8");
    signature = base58(signed.signature);
  } catch (e) {
    return { ok: false, error: e instanceof Error ? e.message : "signature rejected" };
  }
  const grant = await apiFetch<Grant>("/v1/auth/verify", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ wallet, nonce: nonce.data.nonce, signature }),
  });
  if (!grant.ok) return { ok: false, error: grant.error };
  storeToken(grant.data.token);
  return { ok: true };
}

/** True when the stored token still maps to `wallet`. */
export async function sessionIsFor(wallet: string): Promise<boolean> {
  if (!sessionToken()) return false;
  const me = await apiFetch<{ wallet: string }>("/v1/auth/me", { headers: authHeaders() });
  return me.ok && me.data.wallet === wallet;
}

/** Connect Phantom and make sure a session exists for that wallet. */
export async function connectAndSignIn(): Promise<
  { ok: true; wallet: string } | { ok: false; error: string }
> {
  const p = phantom();
  if (!p) return { ok: false, error: "Install Phantom to fuel a listing." };
  const resp = await p.connect();
  const wallet = resp.publicKey.toString();
  if (await sessionIsFor(wallet)) return { ok: true, wallet };
  storeToken(null);
  const res = await signIn(wallet);
  return res.ok ? { ok: true, wallet } : res;
}

export async function signOut(): Promise<void> {
  if (sessionToken()) {
    await apiFetch("/v1/auth/logout", { method: "POST", headers: authHeaders() });
  }
  storeToken(null);
}
