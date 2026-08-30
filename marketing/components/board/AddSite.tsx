"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import { apiFetch, type Project } from "@/lib/api";

type Phantom = {
  isPhantom?: boolean;
  connect: () => Promise<{ publicKey: { toString: () => string } }>;
};

function phantomProvider(): Phantom | null {
  if (typeof window === "undefined") return null;
  const w = window as unknown as { solana?: Phantom; phantom?: { solana?: Phantom } };
  return w.solana?.isPhantom ? w.solana : w.phantom?.solana || null;
}

export default function AddSite() {
  const router = useRouter();
  const [url, setUrl] = useState("");
  const [name, setName] = useState("");
  const [wallet, setWallet] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [msg, setMsg] = useState<string | null>(null);

  async function connect() {
    const p = phantomProvider();
    if (!p) {
      setMsg("Phantom is not installed. You can still list a site without it.");
      return;
    }
    try {
      const resp = await p.connect();
      setWallet(resp.publicKey.toString());
      setMsg(null);
    } catch (e) {
      setMsg(e instanceof Error ? e.message : "Phantom connect failed");
    }
  }

  async function onSubmit(e: React.FormEvent) {
    e.preventDefault();
    setBusy(true);
    setMsg(null);
    try {
      const res = await apiFetch<{ created: boolean; project: Project }>("/v1/projects", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          url: url.trim(),
          display_name: name.trim() || undefined,
          owner_wallet: wallet || undefined,
        }),
      });
      if (!res.ok) {
        setMsg(res.error === "bad_request" ? "That URL could not be listed." : res.error);
        return;
      }
      const handle = res.data.project.handle;
      if (res.data.created) {
        router.push(`/garage/${encodeURIComponent(handle)}`);
      } else {
        setMsg("Already on the board. Opening it.");
        router.push(`/garage/${encodeURIComponent(handle)}`);
      }
    } finally {
      setBusy(false);
    }
  }

  return (
    <section id="add" className="mt-10 rounded-2xl border border-emerald-100 bg-white p-6">
      <p className="k text-forest">Add your site</p>
      <h2 className="mt-2 text-2xl font-bold">Not in the outbid catalog? List it.</h2>
      <p className="mt-2 max-w-xl text-sm text-neutral-600">
        Paste a website. It lands at 0 RP. Fuel it to climb. Duplicate hosts open the existing row.
      </p>
      <form onSubmit={onSubmit} className="mt-5 flex flex-col gap-3 md:flex-row md:items-end">
        <label className="flex-1 text-xs tracking-[0.14em] text-neutral-500">
          URL
          <input
            required
            value={url}
            onChange={(e) => setUrl(e.target.value)}
            placeholder="https://yourcompany.com"
            className="mt-1 w-full rounded-full border border-emerald-100 bg-mint px-4 py-2.5 text-sm text-ink outline-none ring-forest/30 focus:ring-2"
          />
        </label>
        <label className="md:w-56 text-xs tracking-[0.14em] text-neutral-500">
          NAME (OPTIONAL)
          <input
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="Display name"
            className="mt-1 w-full rounded-full border border-emerald-100 bg-mint px-4 py-2.5 text-sm text-ink outline-none ring-forest/30 focus:ring-2"
          />
        </label>
        <button
          type="submit"
          disabled={busy || !url.trim()}
          className="relative inline-flex items-center justify-center overflow-hidden rounded-full bg-forest px-5 py-2.5 text-sm font-semibold uppercase tracking-wide text-white shadow-sm disabled:opacity-50"
        >
          {busy ? "Listing…" : "Add to rank"}
        </button>
      </form>
      <div className="mt-3 flex flex-wrap items-center gap-3 text-xs text-neutral-500">
        <button type="button" onClick={connect} className="underline decoration-forest/40 hover:text-forest">
          {wallet ? `Owner ${wallet.slice(0, 4)}…${wallet.slice(-4)}` : "Attach Phantom as owner (optional)"}
        </button>
        {msg ? <span className="text-forest">{msg}</span> : null}
      </div>
    </section>
  );
}
