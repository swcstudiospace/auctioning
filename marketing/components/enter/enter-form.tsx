"use client";

import { useMemo, useState } from "react";
import { BackLink } from "@/components/chrome/back-link";
import { ShinyButton } from "@/components/magicui/shiny-button";

const STEP = 10;
const MIN = 10;

export default function EnterForm() {
  const [amount, setAmount] = useState(410);
  const [wallet, setWallet] = useState(false);

  const prediction = useMemo(() => {
    if (amount >= 900) return "P3";
    if (amount >= 620) return "P5";
    if (amount >= 410) return "P8";
    return "P12";
  }, [amount]);

  return (
    <main className="mx-auto max-w-3xl px-4 py-8 sm:px-6">
      <BackLink href="/live/" label="BACK TO LIVE RACE" />
      <p className="text-[11px] font-semibold tracking-[0.18em] text-forest">PLACE A BID</p>
      <h1 className="mt-2 text-4xl font-bold tracking-tight">Enter the sprint</h1>
      <p className="mt-3 text-muted">
        Phantom Solana and Whop card are stubs on this marketing origin. No live checkout, no API keys, no wallet signing.
      </p>

      <section className="mt-8 space-y-6 rounded-[28px] bg-white p-6 shadow-[0_12px_32px_rgba(15,40,25,0.05)] sm:p-8">
        <div>
          <h2 className="text-sm font-semibold">1. Connect wallet</h2>
          <div className="mt-3 flex items-center justify-between rounded-2xl border border-line px-4 py-3">
            <div>
              <p className="font-semibold">Phantom</p>
              <p className="text-sm text-muted">Solana · stub only</p>
            </div>
            <button
              type="button"
              onClick={() => setWallet(true)}
              className="rounded-full border border-forest/30 px-4 py-1.5 text-[11px] font-semibold tracking-[0.12em] text-forest"
            >
              {wallet ? "CONNECTED" : "CONNECT"}
            </button>
          </div>
        </div>

        <div>
          <h2 className="text-sm font-semibold">2. RP amount</h2>
          <div className="mt-3 flex items-center justify-center gap-4">
            <button type="button" className="h-10 w-10 rounded-full border border-line" onClick={() => setAmount((n) => Math.max(MIN, n - STEP))} aria-label="Decrease RP">-</button>
            <p className="font-mono text-4xl font-semibold">{amount}</p>
            <button type="button" className="h-10 w-10 rounded-full border border-line" onClick={() => setAmount((n) => n + STEP)} aria-label="Increase RP">+</button>
          </div>
          <p className="mt-2 text-center text-sm text-muted">Bids move in 10 RP increments. $1 = 1 paid RP.</p>
        </div>

        <div>
          <h2 className="text-sm font-semibold">3. Payment</h2>
          <div className="mt-3 flex items-center justify-between rounded-2xl border border-line px-4 py-3">
            <div>
              <p className="font-semibold">Whop Card</p>
              <p className="text-sm text-muted">Card checkout stub — not live</p>
            </div>
            <span className="text-[11px] font-semibold tracking-[0.12em] text-muted">SECURE</span>
          </div>
        </div>

        <div className="rounded-2xl bg-mint px-4 py-4">
          <p className="text-[11px] font-semibold tracking-[0.14em] text-muted">LIVE RANK PREDICTION</p>
          <p className="mt-2 text-sm text-ink">A bid of <span className="font-mono font-semibold text-forest">{amount} RP</span> puts you</p>
          <p className="mt-1 font-mono text-4xl font-bold text-ink">{prediction}</p>
          <p className="mt-1 text-xs text-muted">410 RP is currently a P8 hop. Illustrative only.</p>
        </div>

        <ShinyButton className="w-full rounded-2xl border-forest bg-forest py-3 text-white" onClick={() => undefined}>
          Add RP (stub)
        </ShinyButton>
        <p className="text-center text-xs text-muted">
          Paid RP is consumable utility. Community RP is promotional and non-cashable. RP is for race use only.
        </p>
      </section>
    </main>
  );
}
