"use client";

import { createClient, type Session, type SupabaseClient } from "@supabase/supabase-js";

/**
 * Supabase is used for **authentication only**. Catalog, RP lots and races
 * still live in the Rust API; every read/write goes through `/v1/*` with the
 * Supabase access token as the bearer (see `lib/auth.ts`). Never query
 * Supabase tables from the site — that would bypass the ledger.
 */

const url = (process.env.NEXT_PUBLIC_SUPABASE_URL || "").trim();
const anon = (process.env.NEXT_PUBLIC_SUPABASE_ANON_KEY || "").trim();

export const supabaseEnabled = Boolean(url && anon);

let client: SupabaseClient | null = null;

export function supabase(): SupabaseClient | null {
  if (!supabaseEnabled || typeof window === "undefined") return null;
  if (!client) {
    client = createClient(url, anon, {
      auth: { persistSession: true, autoRefreshToken: true, detectSessionInUrl: true },
    });
  }
  return client;
}

/** Current Supabase session, or null when signed out / not configured. */
export async function supabaseSession(): Promise<Session | null> {
  const c = supabase();
  if (!c) return null;
  const { data } = await c.auth.getSession();
  return data.session ?? null;
}

/** Synchronous best-effort access token for request headers. */
export function supabaseAccessToken(): string | null {
  if (!supabaseEnabled || typeof window === "undefined") return null;
  try {
    const ref = url.replace(/^https?:\/\//, "").split(".")[0];
    const raw = window.localStorage.getItem(`sb-${ref}-auth-token`);
    if (!raw) return null;
    const parsed = JSON.parse(raw) as { access_token?: string; expires_at?: number };
    if (!parsed.access_token) return null;
    if (parsed.expires_at && parsed.expires_at * 1000 < Date.now() - 5_000) return null;
    return parsed.access_token;
  } catch {
    return null;
  }
}

/** Email one-time code / magic link. Redirects back to the current page. */
export async function signInWithEmail(email: string): Promise<{ ok: true } | { ok: false; error: string }> {
  const c = supabase();
  if (!c) return { ok: false, error: "Sign-in is not configured." };
  const { error } = await c.auth.signInWithOtp({
    email,
    options: { emailRedirectTo: window.location.href },
  });
  return error ? { ok: false, error: error.message } : { ok: true };
}

export async function verifyEmailCode(email: string, token: string): Promise<{ ok: true } | { ok: false; error: string }> {
  const c = supabase();
  if (!c) return { ok: false, error: "Sign-in is not configured." };
  const { error } = await c.auth.verifyOtp({ email, token, type: "email" });
  return error ? { ok: false, error: error.message } : { ok: true };
}

export async function supabaseSignOut(): Promise<void> {
  const c = supabase();
  if (c) await c.auth.signOut();
}
