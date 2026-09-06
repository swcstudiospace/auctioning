"use client";

import { useEffect, useState } from "react";
import { apiFetch } from "@/lib/api";
import { authHeaders } from "@/lib/auth";
import {
  signInWithEmail,
  supabase,
  supabaseEnabled,
  supabaseSignOut,
  verifyEmailCode,
} from "@/lib/supabase";

type Me = { wallet: string; principal?: "supabase" | "wallet"; email?: string | null };

/**
 * Supabase email sign-in for the site chrome. Renders nothing when
 * NEXT_PUBLIC_SUPABASE_URL/ANON_KEY are unset so wallet-only deploys are
 * unaffected. After sign-in every `/v1/*` call carries the Supabase access
 * token (see `authHeaders`) and the API answers as the user's ledger wallet.
 */
export default function AccountMenu() {
  const [me, setMe] = useState<Me | null>(null);
  const [open, setOpen] = useState(false);
  const [email, setEmail] = useState("");
  const [code, setCode] = useState("");
  const [stage, setStage] = useState<"email" | "code">("email");
  const [busy, setBusy] = useState(false);
  const [msg, setMsg] = useState<string | null>(null);

  async function refresh() {
    const headers = authHeaders();
    if (!headers.authorization) {
      setMe(null);
      return;
    }
    const res = await apiFetch<Me>("/v1/auth/me", { headers });
    setMe(res.ok ? res.data : null);
  }

  useEffect(() => {
    if (!supabaseEnabled) return;
    void refresh();
    const c = supabase();
    const sub = c?.auth.onAuthStateChange(() => {
      void refresh();
    });
    return () => sub?.data.subscription.unsubscribe();
  }, []);

  if (!supabaseEnabled) return null;

  async function sendCode(e: React.FormEvent) {
    e.preventDefault();
    setBusy(true);
    setMsg(null);
    const res = await signInWithEmail(email.trim());
    setBusy(false);
    if (!res.ok) {
      setMsg(res.error);
      return;
    }
    setStage("code");
    setMsg("Check your inbox for a code or link.");
  }

  async function submitCode(e: React.FormEvent) {
    e.preventDefault();
    setBusy(true);
    setMsg(null);
    const res = await verifyEmailCode(email.trim(), code.trim());
    setBusy(false);
    if (!res.ok) {
      setMsg(res.error);
      return;
    }
    setOpen(false);
    setStage("email");
    setCode("");
    await refresh();
  }

  async function signOut() {
    setBusy(true);
    await apiFetch("/v1/auth/logout", { method: "POST", headers: authHeaders() });
    await supabaseSignOut();
    setBusy(false);
    setMe(null);
    setOpen(false);
  }

  const label = me?.email || (me ? `${me.wallet.slice(0, 6)}…` : "Sign in");

  return (
    <div className="relative">
      <button
        type="button"
        onClick={() => setOpen((v) => !v)}
        className="rounded-full border border-emerald-200 bg-white px-3 py-1 text-xs font-semibold text-forest hover:bg-emerald-50"
        aria-expanded={open}
      >
        {label}
      </button>
      {open ? (
        <div className="absolute right-0 z-50 mt-2 w-72 rounded-2xl border border-emerald-100 bg-white p-4 shadow-lg">
          {me ? (
            <div className="space-y-3 text-sm">
              <p className="font-semibold">{me.email || "Signed in"}</p>
              <p className="break-all font-mono text-[11px] text-neutral-500">ledger {me.wallet}</p>
              <button
                type="button"
                onClick={signOut}
                disabled={busy}
                className="w-full rounded-lg border border-emerald-200 px-3 py-2 text-xs font-semibold hover:bg-emerald-50 disabled:opacity-50"
              >
                Sign out
              </button>
            </div>
          ) : stage === "email" ? (
            <form onSubmit={sendCode} className="space-y-3 text-sm">
              <p className="font-semibold">Sign in with email</p>
              <input
                type="email"
                required
                value={email}
                onChange={(e) => setEmail(e.target.value)}
                placeholder="you@company.com"
                className="w-full rounded-lg border border-emerald-200 px-3 py-2 text-sm"
              />
              <button
                type="submit"
                disabled={busy}
                className="w-full rounded-lg bg-forest px-3 py-2 text-xs font-semibold text-white disabled:opacity-50"
              >
                {busy ? "Sending…" : "Send code"}
              </button>
              <p className="text-[11px] text-neutral-500">
                Weekly free RP and support go to the account you sign in with. Phantom still works for on-chain RP.
              </p>
            </form>
          ) : (
            <form onSubmit={submitCode} className="space-y-3 text-sm">
              <p className="font-semibold">Enter the code sent to {email}</p>
              <input
                inputMode="numeric"
                required
                value={code}
                onChange={(e) => setCode(e.target.value)}
                placeholder="123456"
                className="w-full rounded-lg border border-emerald-200 px-3 py-2 font-mono text-sm"
              />
              <button
                type="submit"
                disabled={busy}
                className="w-full rounded-lg bg-forest px-3 py-2 text-xs font-semibold text-white disabled:opacity-50"
              >
                {busy ? "Checking…" : "Sign in"}
              </button>
              <button
                type="button"
                onClick={() => setStage("email")}
                className="w-full text-[11px] text-neutral-500 underline"
              >
                Use a different email
              </button>
            </form>
          )}
          {msg ? <p className="mt-3 text-[11px] text-neutral-600">{msg}</p> : null}
        </div>
      ) : null}
    </div>
  );
}
