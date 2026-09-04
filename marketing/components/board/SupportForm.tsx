"use client";

import { FormEvent, useState } from "react";
import { claimWeekly, getRp, supportProject, type Project, type RpView } from "@/lib/api";

type Phantom = {
  isPhantom?: boolean;
  connect: () => Promise<{ publicKey: { toString: () => string } }>;
};

function phantom(): Phantom | null {
  if (typeof window === "undefined") return null;
  const w = window as unknown as { solana?: Phantom; phantom?: { solana?: Phantom } };
  return w.solana?.isPhantom ? w.solana : w.phantom?.solana || null;
}

function fmtRp(n: number): string {
  return n.toLocaleString() + " RP";
}

export default function SupportForm({ project }: { project: Project }) {
  const [wallet, setWallet] = useState<string | null>(null);
  const [rp, setRp] = useState<RpView | null>(null);
  const [amount, setAmount] = useState(1);
  const [reason, setReason] = useState("");
  const [busy, setBusy] = useState(false);
  const [msg, setMsg] = useState<string | null>(null);

  async function connect(): Promise<string | null> {
    const p = phantom();
    if (!p) {
      setMsg("Install Phantom to fuel a listing.");
      return null;
    }
    const resp = await p.connect();
    const addr = resp.publicKey.toString();
    setWallet(addr);
    const view = await getRp(addr);
    if (view.ok) setRp(view.data);
    return addr;
  }

  async function onSubmit(e: FormEvent) {
    e.preventDefault();
    setBusy(true);
    setMsg(null);
    try {
      const addr = wallet || (await connect());
      if (!addr) return;
      let view = rp;
      if (!view) {
        const got = await getRp(addr);
        if (got.ok) view = got.data;
      }
      const available = (view?.free_rp ?? 0) + (view?.paid_rp ?? 0);
      if (available < amount) {
        const claimed = await claimWeekly(addr);
        if (claimed.ok) {
          const again = await getRp(addr);
          if (again.ok) {
            view = again.data;
            setRp(again.data);
          }
        }
      }
      const have = (view?.free_rp ?? 0) + (view?.paid_rp ?? 0);
      if (have < amount) {
        setMsg(`Need ${fmtRp(amount)}; wallet has ${fmtRp(have)}. Claim weekly or buy paid RP.`);
        return;
      }
      const res = await supportProject(project.handle, {
        wallet: addr,
        amount,
        reason: reason.trim() || undefined,
      });
      if (!res.ok) {
        setMsg(res.error === "insufficient_funds" ? "Not enough RP." : res.error);
        return;
      }
      setMsg(`Put ${fmtRp(amount)} on ${project.display_name || project.handle}. Total ${fmtRp(res.data.project_total_rp)}.`);
      const again = await getRp(addr);
      if (again.ok) setRp(again.data);
    } catch (err) {
      setMsg(err instanceof Error ? err.message : "Support failed");
    } finally {
      setBusy(false);
    }
  }

  return (
    <form onSubmit={onSubmit} className="mt-4 rounded-2xl border border-emerald-100 bg-white p-4">
      <p className="k text-forest">Fuel this listing</p>
      <h3 className="mt-1 text-lg font-semibold">{project.display_name || project.handle}</h3>
      <p className="mt-1 text-xs text-neutral-500">
        RP is spent onto this handle. Weekly 50 is community; paid RP is $1 = 1.
      </p>
      <label className="mt-3 block text-xs uppercase tracking-wide text-neutral-500">
        Amount (RP)
        <input
          type="number"
          min={1}
          value={amount}
          onChange={(e) => setAmount(Math.max(1, Number(e.target.value) || 1))}
          className="mt-1 w-full rounded-lg border border-emerald-100 px-3 py-2 text-sm"
        />
      </label>
      <label className="mt-3 block text-xs uppercase tracking-wide text-neutral-500">
        Reason (optional)
        <input
          value={reason}
          onChange={(e) => setReason(e.target.value.slice(0, 128))}
          className="mt-1 w-full rounded-lg border border-emerald-100 px-3 py-2 text-sm"
        />
      </label>
      <button
        type="submit"
        disabled={busy}
        className="mt-4 w-full rounded-full bg-forest px-5 py-2.5 text-sm font-semibold uppercase tracking-wide text-white disabled:opacity-50"
      >
        {busy ? "Supporting…" : `Put ${fmtRp(amount)} on this listing`}
      </button>
      {wallet ? <p className="mt-2 font-mono text-[10px] text-neutral-400">{wallet.slice(0, 4)}…{wallet.slice(-4)}</p> : null}
      {msg ? <p className="mt-2 text-sm text-neutral-600">{msg}</p> : null}
    </form>
  );
}
