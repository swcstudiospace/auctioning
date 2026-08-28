"use client";

import { useEffect, useMemo, useState } from "react";
import { MagicCard } from "@/components/magic/MagicCard";
import { getJson, postJson, type GridSlot } from "@/lib/api";
import { predictRank, seedGrid } from "@/lib/sim";

type Phantom = {
  isPhantom?: boolean;
  connect: () => Promise<{ publicKey: { toString: () => string } }>;
};

function phantomProvider(): Phantom | null {
  if (typeof window === "undefined") return null;
  const w = window as unknown as { solana?: Phantom; phantom?: { solana?: Phantom } };
  return w.solana?.isPhantom ? w.solana : w.phantom?.solana || null;
}

export default function EnterRace() {
  const [grid, setGrid] = useState<GridSlot[]>(() => seedGrid());
  const [amount, setAmount] = useState(410);
  const [wallet, setWallet] = useState<string | null>(null);
  const [status, setStatus] = useState("Connect Phantom or set a bid to see predicted rank.");
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    getJson<{ grid: GridSlot[] }>("/v1/grid").then((res) => {
      if (res?.grid?.length) setGrid(res.grid);
    });
  }, []);

  const rank = useMemo(() => predictRank(grid, amount), [grid, amount]);

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
    const prepared = await postJson<{ tx_base64?: string; note?: string }>("/v1/onchain/prepare-log-paid", {
      wallet,
      rp_amount: amount,
      lamports_paid: amount * 1_000_000,
      memo: "auctioning.lol bid",
      current_receipt_count: 0,
    });
    if (prepared?.tx_base64) {
      setStatus(prepared.note || "Unsigned tx ready. Sign it in Phantom to log paid RP on-chain.");
    } else {
      setStatus(`No API tx yet. Local prediction still holds: ${amount} RP ≈ P${rank}.`);
    }
    setBusy(false);
  }

  return (
    <main className="mx-auto max-w-6xl px-6 py-10">
      <h1 className="text-4xl font-bold">Place a bid</h1>
      <p className="mt-3 max-w-2xl text-neutral-600">
        Paid RP logs to the private ledger and on-chain. Community RP stays off-chain and non-cashable.
      </p>
      <div className="mt-8 grid gap-4 md:grid-cols-2">
        <MagicCard>
          <div className="k">Phantom</div>
          <h3 className="mt-2 text-2xl font-semibold">Solana</h3>
          <p className="mt-2 text-sm text-neutral-600">Connect wallet. $1 equivalent = 1 paid RP, logged on-chain.</p>
          <button className="mt-4 rounded-full bg-forest px-5 py-2 text-sm font-semibold uppercase text-white" type="button" onClick={connectPhantom}>
            {wallet ? "Phantom connected" : "Connect Phantom"}
          </button>
        </MagicCard>
        <MagicCard>
          <div className="k">Whop</div>
          <h3 className="mt-2 text-2xl font-semibold">Card</h3>
          <p className="mt-2 text-sm text-neutral-600">Card / Apple Pay posts to the Whop webhook, then authority log_paid_rp. Checkout URL comes from env, never invented.</p>
          {process.env.NEXT_PUBLIC_WHOP_CHECKOUT_URL ? (
            <a className="mt-4 inline-block rounded-full bg-forest px-5 py-2 text-sm font-semibold uppercase text-white" href={process.env.NEXT_PUBLIC_WHOP_CHECKOUT_URL}>
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
        <div className="k">Rank prediction</div>
        <label className="mt-3 block text-sm">
          Bid RP
          <input
            className="mt-2 w-40 rounded-lg border border-emerald-200 px-3 py-2 font-mono"
            type="number"
            min={10}
            step={10}
            value={amount}
            onChange={(e) => setAmount(Math.max(10, Number(e.target.value) || 0))}
          />
        </label>
        <p className="mt-3">
          A bid of <b>{amount} RP</b> currently puts a new listing around <b className="text-forest">P{rank}</b> on this grid.
        </p>
        <button
          className="mt-4 rounded-full bg-forest px-5 py-2 text-sm font-semibold uppercase text-white disabled:opacity-50"
          type="button"
          disabled={busy}
          onClick={placePaidBid}
        >
          Log paid RP
        </button>
        <p className="mt-3 text-sm text-neutral-600">{status}</p>
      </MagicCard>
    </main>
  );
}
