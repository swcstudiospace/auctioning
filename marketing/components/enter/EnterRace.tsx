"use client";

import { useState } from "react";
import { MagicCard } from "@/components/magic/MagicCard";
import { WHOP_CHECKOUT_URL, postJson } from "@/lib/api";

type Phantom = {
  isPhantom?: boolean;
  connect: () => Promise<{ publicKey: { toString: () => string } }>;
  signAndSendTransaction: (tx: unknown) => Promise<{ signature?: string }>;
};

function phantomProvider(): Phantom | null {
  if (typeof window === "undefined") return null;
  const w = window as unknown as { solana?: Phantom; phantom?: { solana?: Phantom } };
  return w.solana?.isPhantom ? w.solana : w.phantom?.solana || null;
}

async function sendPreparedTx(p: Phantom, txBase64: string): Promise<string> {
  const txBytes = Uint8Array.from(atob(txBase64), (c) => c.charCodeAt(0));
  const w = window as unknown as { solanaWeb3?: { Transaction: { from: (b: Uint8Array) => unknown } } };
  const tx = w.solanaWeb3?.Transaction ? w.solanaWeb3.Transaction.from(txBytes) : { transaction: txBytes };
  const result = await p.signAndSendTransaction(tx);
  if (!result?.signature) throw new Error("Phantom returned no signature");
  return result.signature;
}

export default function EnterRace() {
  const [amount, setAmount] = useState(10);
  const [wallet, setWallet] = useState<string | null>(null);
  const [status, setStatus] = useState("Connect Phantom, then log paid RP on-chain.");
  const [busy, setBusy] = useState(false);
  const [sig, setSig] = useState<string | null>(null);

  async function connectPhantom() {
    const p = phantomProvider();
    if (!p) {
      setStatus("Phantom is not installed in this browser.");
      return;
    }
    try {
      const resp = await p.connect();
      setWallet(resp.publicKey.toString());
      setStatus(`Connected ${resp.publicKey.toString().slice(0, 4)}…${resp.publicKey.toString().slice(-4)}`);
    } catch (e) {
      setStatus(e instanceof Error ? e.message : "Phantom connect failed");
    }
  }

  async function placePaidBid() {
    if (!wallet) {
      setStatus("Connect Phantom first.");
      return;
    }
    setBusy(true);
    setSig(null);
    try {
      const prepared = await postJson<{
        tx_base64?: string;
        note?: string;
        program_id?: string;
        receipt_pda?: string;
      }>("/v1/onchain/prepare-log-paid", {
        wallet,
        rp_amount: amount,
        lamports_paid: amount * 1_000_000,
        memo: "auctioning.lol paid RP",
        current_receipt_count: 0,
      });
      if (!prepared?.tx_base64) {
        setStatus(prepared?.note || "prepare-log-paid returned no transaction. Check PROGRAM_ID / RPC on the API.");
        return;
      }
      const p = phantomProvider();
      if (!p) {
        setStatus("Phantom went away before signing.");
        return;
      }
      const signature = await sendPreparedTx(p, prepared.tx_base64);
      setSig(signature);
      setStatus(`Logged on-chain. ${prepared.note || ""}`);
    } catch (e) {
      setStatus(e instanceof Error ? e.message : "log_paid_rp failed");
    } finally {
      setBusy(false);
    }
  }

  return (
    <main className="mx-auto max-w-6xl px-6 py-10">
      <h1 className="text-4xl font-bold">Place a bid</h1>
      <p className="mt-3 max-w-2xl text-neutral-600">
        Paid RP: $1 = 1, signed through Phantom as log_paid_rp. Community RP stays off-chain and non-cashable.
      </p>
      <div className="mt-8 grid gap-4 md:grid-cols-2">
        <MagicCard>
          <div className="k">Phantom</div>
          <h3 className="mt-2 text-2xl font-semibold">Solana</h3>
          <p className="mt-2 text-sm text-neutral-600">Connect wallet, then sign the unsigned tx from prepare-log-paid.</p>
          <button className="mt-4 rounded-full bg-forest px-5 py-2 text-sm font-semibold uppercase text-white" type="button" onClick={connectPhantom}>
            {wallet ? "Phantom connected" : "Connect Phantom"}
          </button>
        </MagicCard>
        <MagicCard>
          <div className="k">Whop</div>
          <h3 className="mt-2 text-2xl font-semibold">Card</h3>
          <p className="mt-2 text-sm text-neutral-600">Fiat hits the Whop webhook, then an authority can submit log_paid_rp. Checkout URL comes from env, never invented.</p>
          {WHOP_CHECKOUT_URL ? (
            <a className="mt-4 inline-block rounded-full bg-forest px-5 py-2 text-sm font-semibold uppercase text-white" href={WHOP_CHECKOUT_URL}>
              Pay with Whop
            </a>
          ) : (
            <button className="mt-4 rounded-full bg-neutral-200 px-5 py-2 text-sm font-semibold uppercase text-neutral-500" type="button" disabled>
              Whop checkout not configured
            </button>
          )}
        </MagicCard>
      </div>
      <MagicCard className="mt-4">
        <div className="k">LOG PAID RP</div>
        <label className="mt-3 block text-sm">
          Paid RP
          <input
            className="mt-2 w-40 rounded-lg border border-emerald-200 px-3 py-2 font-mono"
            type="number"
            min={1}
            step={1}
            value={amount}
            onChange={(e) => setAmount(Math.max(1, Number(e.target.value) || 0))}
          />
        </label>
        <p className="mt-3 text-sm text-neutral-600">
          Builds an unsigned Anchor log_paid_rp transaction, then Phantom signs and broadcasts it.
        </p>
        <button
          className="mt-4 rounded-full bg-forest px-5 py-2 text-sm font-semibold uppercase text-white disabled:opacity-50"
          type="button"
          disabled={busy}
          onClick={placePaidBid}
        >
          {busy ? "Waiting on Phantom…" : "Sign log_paid_rp"}
        </button>
        <p className="mt-3 text-sm text-neutral-600">{status}</p>
        {sig ? (
          <a className="mt-2 inline-block text-xs text-forest" href={`https://solscan.io/tx/${sig}`} target="_blank" rel="noreferrer">
            {sig}
          </a>
        ) : null}
      </MagicCard>
    </main>
  );
}
