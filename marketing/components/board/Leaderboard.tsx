"use client";

import { FormEvent, useCallback, useEffect, useMemo, useState } from "react";
import { usePathname, useRouter, useSearchParams } from "next/navigation";
import {
  WHOP_CHECKOUT_URL,
  amountToClaimFirst,
  claimWeekly,
  getJson,
  getRp,
  listProjects,
  predictRank,
  supportProject,
  type GridSlot,
  type Project,
  type ProjectList,
  type RpView,
} from "@/lib/api";
import CompanyIcon from "@/components/chrome/CompanyIcon";
import GarageCard, { statsFromProject } from "@/components/garage/GarageCard";
import AddSite from "@/components/board/AddSite";
import { deriveBadge, gapToNextOnPage } from "@/lib/race";
import { BlurFade } from "@/components/magic/BlurFade";
import { NumberTicker } from "@/components/magic/NumberTicker";

type PhantomProvider = {
  isPhantom?: boolean;
  publicKey?: { toString(): string };
  connect: (opts?: { onlyIfTrusted?: boolean }) => Promise<{ publicKey: { toString(): string } }>;
};

function phantom(): PhantomProvider | null {
  if (typeof window === "undefined") return null;
  const w = window as unknown as {
    solana?: PhantomProvider;
    phantom?: { solana?: PhantomProvider };
  };
  if (w.solana?.isPhantom) return w.solana;
  return w.phantom?.solana ?? null;
}

function hostOf(url: string | null): string {
  if (!url) return "";
  try {
    return new URL(url).host.replace(/^www\./, "");
  } catch {
    return url.replace(/^https?:\/\//, "").replace(/\/$/, "");
  }
}

function fmtRp(n: number): string {
  return `${n.toLocaleString("en-US")} RP`;
}

function tagLabel(tag: string): string {
  return tag.replace(/-/g, " ");
}

function shortWallet(w: string): string {
  if (w.length < 10) return w;
  return `${w.slice(0, 4)}…${w.slice(-4)}`;
}

export default function Leaderboard({ initial }: { initial?: ProjectList | null }) {
  const router = useRouter();
  const pathname = usePathname();
  const searchParams = useSearchParams();
  const page = Math.max(1, Number(searchParams.get("page") || "1") || 1);
  const tag = searchParams.get("tag") || "";
  const qParam = searchParams.get("q") || "";

  const [qInput, setQInput] = useState(qParam);
  const [projects, setProjects] = useState<Project[]>(initial?.projects || []);
  const [tags, setTags] = useState<string[]>(initial?.tags || []);
  const [total, setTotal] = useState(initial?.total || 0);
  const [perPage, setPerPage] = useState(initial?.per_page || 50);
  const [leaderRp, setLeaderRp] = useState(initial?.projects?.[0]?.total_rp ?? 0);
  const [loading, setLoading] = useState(!initial);
  const [error, setError] = useState<string | null>(null);

  const [wallet, setWallet] = useState<string | null>(null);
  const [rp, setRp] = useState<RpView | null>(null);
  const [walletMsg, setWalletMsg] = useState<string | null>(null);

  const [selected, setSelected] = useState<Project | null>(null);
  const [gridByHandle, setGridByHandle] = useState<Record<string, GridSlot>>({});
  const [hoverHandle, setHoverHandle] = useState<string | null>(null);
  const [amount, setAmount] = useState(1);
  const [reason, setReason] = useState("");
  const [busy, setBusy] = useState(false);
  const [flash, setFlash] = useState<string | null>(null);

  const setQuery = useCallback(
    (next: { page?: number; tag?: string; q?: string }) => {
      const sp = new URLSearchParams();
      const nextPage = next.page ?? page;
      const nextTag = next.tag === undefined ? tag : next.tag;
      const nextQ = next.q === undefined ? qParam : next.q;
      if (nextPage > 1) sp.set("page", String(nextPage));
      if (nextTag) sp.set("tag", nextTag);
      if (nextQ) sp.set("q", nextQ);
      const qs = sp.toString();
      router.push(qs ? `${pathname}?${qs}` : pathname, { scroll: false });
    },
    [page, tag, qParam, pathname, router],
  );

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    const res = await listProjects({ page, per_page: 50, tag: tag || undefined, q: qParam || undefined });
    if (!res.ok) {
      setProjects([]);
      setError(res.status === 0 ? "Catalog API is unreachable." : `Catalog API error (${res.status}).`);
      setLoading(false);
      return;
    }
    setProjects(res.data.projects || []);
    setTotal(res.data.total ?? 0);
    setPerPage(res.data.per_page ?? 50);
    if (res.data.tags) setTags(res.data.tags);
    setLoading(false);
  }, [page, tag, qParam]);

  useEffect(() => {
    let cancel = false;
    getJson<{ grid: GridSlot[] }>("/v1/grid").then((res) => {
      if (cancel || !res?.grid) return;
      const map: Record<string, GridSlot> = {};
      for (const slot of res.grid) map[slot.handle] = slot;
      setGridByHandle(map);
    });
    return () => {
      cancel = true;
    };
  }, []);

  useEffect(() => {
    setQInput(qParam);
  }, [qParam]);

  useEffect(() => {
    load();
  }, [load]);

  useEffect(() => {
    let cancel = false;
    async function pullLeader() {
      const res = await listProjects({ page: 1, per_page: 1, tag: tag || undefined, q: qParam || undefined });
      if (cancel || !res.ok) return;
      setLeaderRp(res.data.projects[0]?.total_rp ?? 0);
    }
    pullLeader();
    return () => {
      cancel = true;
    };
  }, [tag, qParam]);

  useEffect(() => {
    const t = setTimeout(() => {
      if (qInput !== qParam) setQuery({ page: 1, q: qInput.trim() });
    }, 350);
    return () => clearTimeout(t);
  }, [qInput, qParam, setQuery]);

  const claimCost = amountToClaimFirst(leaderRp, 0);
  const pages = Math.max(1, Math.ceil(total / perPage));

  const predicted = useMemo(() => {
    if (!selected) return null;
    return predictRank(projects, selected.handle, selected.total_rp + Math.max(0, amount));
  }, [selected, projects, amount]);

  async function refreshRp(addr: string) {
    const res = await getRp(addr);
    if (res.ok) setRp(res.data);
    return res;
  }

  async function connectWallet(): Promise<string | null> {
    setWalletMsg(null);
    const p = phantom();
    if (!p) {
      setWalletMsg("Phantom not found. Install the Phantom browser extension.");
      return null;
    }
    try {
      const resp = await p.connect();
      const addr = resp.publicKey.toString();
      setWallet(addr);
      await refreshRp(addr);
      return addr;
    } catch (e) {
      setWalletMsg(e instanceof Error ? e.message : "Phantom connect rejected");
      return null;
    }
  }

  async function ensureBalance(addr: string): Promise<RpView | null> {
    const first = await getRp(addr);
    if (!first.ok) {
      setWalletMsg(first.error);
      return null;
    }
    let view = first.data;
    if ((view.free_rp ?? 0) + (view.paid_rp ?? 0) > 0) {
      setRp(view);
      return view;
    }
    const claimed = await claimWeekly(addr);
    if (claimed.ok) {
      setFlash(`Claimed ${claimed.data.amount ?? 0} weekly RP.`);
    } else if (claimed.status === 429) {
      setWalletMsg("Weekly RP already claimed and balance is 0.");
    } else {
      setWalletMsg(claimed.error);
    }
    const again = await getRp(addr);
    if (again.ok) {
      setRp(again.data);
      return again.data;
    }
    setRp(view);
    return view;
  }

  async function openSupport(project: Project, preferClaim = false) {
    setSelected(project);
    setFlash(null);
    const defaultAmt = preferClaim
      ? amountToClaimFirst(leaderRp, project.total_rp)
      : Math.max(1, amountToClaimFirst(leaderRp, project.total_rp));
    setAmount(defaultAmt);
    const addr = wallet || (await connectWallet());
    if (addr) await ensureBalance(addr);
  }

  async function onSupport(e: FormEvent) {
    e.preventDefault();
    if (!selected) return;
    setBusy(true);
    setFlash(null);
    setWalletMsg(null);
    try {
      const addr = wallet || (await connectWallet());
      if (!addr) return;
      const view = await ensureBalance(addr);
      const available = (view?.free_rp ?? 0) + (view?.paid_rp ?? 0);
      if (available < amount) {
        setWalletMsg(`Need ${fmtRp(amount)}; wallet has ${fmtRp(available)}.`);
        return;
      }
      const res = await supportProject(selected.handle, {
        wallet: addr,
        amount,
        reason: reason.trim() || undefined,
      });
      if (!res.ok) {
        setWalletMsg(
          res.error === "insufficient_funds"
            ? "Not enough RP. Claim weekly or add more, then retry."
            : res.error,
        );
        return;
      }
      setFlash(
        `Supported ${selected.display_name || selected.handle} with ${fmtRp(amount)}. New total ${fmtRp(res.data.project_total_rp)}.`,
      );
      setSelected({ ...selected, total_rp: res.data.project_total_rp });
      await refreshRp(addr);
      await load();
    } finally {
      setBusy(false);
    }
  }

  function onClaimCta() {
    const target = selected || projects[0];
    if (!target) {
      setWalletMsg("Board is empty — nothing to claim.");
      return;
    }
    openSupport(target, true);
  }

  return (
    <main className={`mx-auto max-w-6xl px-6 py-10 ${hoverHandle ? "pb-72 lg:pb-10" : ""}`}>
      <section id="claim" className="flex flex-col gap-6 md:flex-row md:items-end md:justify-between">
        <BlurFade>
          <p className="k text-forest">Play to rank</p>
          <h1 className="mt-2 text-4xl font-bold leading-tight md:text-5xl">
            Company leaderboard
          </h1>
          <p className="mt-3 max-w-xl text-neutral-600">
            Fuel a listing with RP to climb the board. Rank is earned, never bought in dollars.
          </p>
        </BlurFade>
        <div className="flex flex-wrap items-center gap-3">
          <button
            type="button"
            onClick={onClaimCta}
            className="relative inline-flex items-center justify-center overflow-hidden rounded-full bg-forest px-5 py-2.5 text-sm font-semibold uppercase tracking-wide text-white shadow-sm bg-[linear-gradient(110deg,#3E8E62,45%,#9AE6B4,55%,#3E8E62)] bg-[length:200%_100%] animate-shine"
          >
            Claim #1 for {fmtRp(claimCost)}+
          </button>
          {WHOP_CHECKOUT_URL ? (
            <a
              href={WHOP_CHECKOUT_URL}
              className="rounded-full border border-forest px-5 py-2.5 text-sm font-semibold uppercase tracking-wide text-forest"
            >
              Buy RP
            </a>
          ) : null}
          <button
            type="button"
            onClick={() => connectWallet()}
            className="rounded-full border border-emerald-200 bg-white px-4 py-2 text-xs font-semibold uppercase tracking-wide text-neutral-700"
          >
            {wallet ? shortWallet(wallet) : "Connect Phantom"}
          </button>
        </div>
      </section>

      {rp && wallet ? (
        <p className="mt-4 text-sm text-neutral-600">
          {shortWallet(wallet)} · free {fmtRp(rp.free_rp)} · paid {fmtRp(rp.paid_rp)}
        </p>
      ) : null}
      {walletMsg ? <p className="mt-2 text-sm text-red-700">{walletMsg}</p> : null}
      {flash ? <p className="mt-2 text-sm text-forest">{flash}</p> : null}

      <AddSite />

      <div className="mt-8 flex flex-col gap-4 md:flex-row md:items-center">
        <input
          value={qInput}
          onChange={(e) => setQInput(e.target.value)}
          placeholder="Search name, handle, blurb…"
          className="w-full rounded-full border border-emerald-100 bg-white px-4 py-2.5 text-sm outline-none ring-forest/30 focus:ring-2 md:max-w-sm"
        />
        <p className="text-xs uppercase tracking-[0.16em] text-neutral-500">
          {loading ? (
            "Loading…"
          ) : (
            <>
              <NumberTicker value={total} /> companies · 50 / page
            </>
          )}
        </p>
      </div>

      <div className="mt-5 flex flex-wrap gap-2">
        <button
          type="button"
          onClick={() => setQuery({ page: 1, tag: "" })}
          className={`chip ${!tag ? "bg-forest text-white" : ""}`}
        >
          All
        </button>
        {tags.map((t) => (
          <button
            key={t}
            type="button"
            onClick={() => setQuery({ page: 1, tag: t === tag ? "" : t })}
            className={`chip ${tag === t ? "bg-forest text-white" : ""}`}
          >
            {tagLabel(t)}
          </button>
        ))}
      </div>

      {error ? (
        <div className="card mt-8 p-8">
          <p className="text-lg font-semibold">Board unavailable</p>
          <p className="mt-2 text-neutral-600">{error}</p>
          <p className="mt-2 text-sm text-neutral-500">No placeholder companies. Retry when the catalog API is up.</p>
        </div>
      ) : null}

      {!error && !loading && projects.length === 0 ? (
        <div className="card mt-8 p-8">
          <p className="text-lg font-semibold">No companies on this page</p>
          <p className="mt-2 text-neutral-600">
            {qParam || tag ? "Nothing matched that filter." : "The catalog is empty."}
          </p>
        </div>
      ) : null}

      {!error && projects.length > 0 ? (
        <div className="mt-8 grid items-start gap-6 lg:grid-cols-[minmax(0,1fr)_21rem]">
          <ol className="divide-y divide-emerald-100 rounded-2xl border border-emerald-100 bg-white">
            {projects.map((p, i) => {
              const active = selected?.handle === p.handle || hoverHandle === p.handle;
              const gap = gapToNextOnPage(projects, i);
              const badge = deriveBadge(p, gridByHandle[p.handle] || null, gap);
              return (
                <li
                  key={p.handle}
                  className={`grid grid-cols-[auto_1fr_auto] items-center gap-4 px-4 py-4 md:grid-cols-[4rem_3rem_1fr_auto] ${active ? "bg-emerald-50/70" : ""}`}
                  onMouseEnter={() => setHoverHandle(p.handle)}
                  onFocus={() => setHoverHandle(p.handle)}
                >
                  <span className="w-10 text-right text-sm font-semibold text-neutral-500">#{p.rank}</span>
                  <CompanyIcon url={p.url} name={p.display_name || p.handle} size={40} />
                  <div className="min-w-0">
                    <div className="flex flex-wrap items-baseline gap-2">
                      <span className="font-semibold">{p.display_name || p.handle}</span>
                      {p.tags[0] ? <span className="chip">{tagLabel(p.tags[0])}</span> : null}
                      {badge ? <span className="chip">{badge.replace(/_/g, " ")}</span> : null}
                      {p.url ? (
                        <a
                          href={p.url}
                          target="_blank"
                          rel="noreferrer"
                          className="truncate text-xs text-forest"
                        >
                          {hostOf(p.url)}
                        </a>
                      ) : null}
                    </div>
                    {p.blurb ? (
                      <p className="mt-1 line-clamp-2 text-sm text-neutral-600">{p.blurb}</p>
                    ) : null}
                  </div>
                  <div className="flex flex-col items-end gap-2">
                    <span className="text-sm font-semibold text-forest">{fmtRp(p.total_rp)}</span>
                    <span className="font-mono text-[11px] text-neutral-400">GAP {gap}</span>
                    <button
                      type="button"
                      onClick={() => openSupport(p)}
                      className="rounded-full border border-forest px-3 py-1 text-[11px] font-semibold uppercase tracking-wide text-forest"
                    >
                      Support
                    </button>
                  </div>
                </li>
              );
            })}
          </ol>
          {(() => {
            const brief =
              projects.find((p) => p.handle === hoverHandle) || projects[0];
            const gap = gapToNextOnPage(
              projects,
              Math.max(0, projects.findIndex((p) => p.handle === brief.handle)),
            );
            return (
              <>
                <aside className="hidden lg:sticky lg:top-24 lg:block">
                  <p className="k mb-2 text-forest">Briefing</p>
                  <GarageCard compact stats={statsFromProject(brief, gridByHandle[brief.handle] || null, gap)} />
                </aside>
                {hoverHandle ? (
                  <div className="fixed inset-x-4 bottom-4 z-40 lg:hidden">
                    <GarageCard compact stats={statsFromProject(brief, gridByHandle[brief.handle] || null, gap)} />
                  </div>
                ) : null}
              </>
            );
          })()}
        </div>
      ) : null}

      {selected ? (
        <form onSubmit={onSupport} className="card mt-6 p-6">
          <div className="flex flex-wrap items-start justify-between gap-4">
            <div>
              <p className="k">Support with RP</p>
              <h2 className="mt-1 text-xl font-semibold">{selected.display_name || selected.handle}</h2>
              <p className="mt-1 text-sm text-neutral-600">
                Now {fmtRp(selected.total_rp)}
                {predicted ? ` · estimated rank #${predicted} on this page after this support` : ""}
              </p>
            </div>
            <button type="button" onClick={() => setSelected(null)} className="text-xs uppercase tracking-wide text-neutral-500">
              Close
            </button>
          </div>
          <div className="mt-4 grid gap-3 md:grid-cols-[1fr_1fr_auto]">
            <label className="text-xs uppercase tracking-wide text-neutral-500">
              Amount (RP)
              <input
                type="number"
                min={1}
                value={amount}
                onChange={(e) => setAmount(Math.max(1, Number(e.target.value) || 1))}
                className="mt-1 w-full rounded-lg border border-emerald-100 px-3 py-2 text-sm"
              />
            </label>
            <label className="text-xs uppercase tracking-wide text-neutral-500">
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
              className="self-end rounded-full bg-forest px-5 py-2.5 text-sm font-semibold uppercase tracking-wide text-white disabled:opacity-50"
            >
              {busy ? "Supporting…" : `Support ${fmtRp(amount)}`}
            </button>
          </div>
        </form>
      ) : null}

      {pages > 1 && !error ? (
        <nav className="mt-8 flex flex-wrap items-center justify-between gap-3 text-sm">
          <button
            type="button"
            disabled={page <= 1}
            onClick={() => setQuery({ page: page - 1 })}
            className="rounded-full border border-emerald-200 px-4 py-2 disabled:opacity-40"
          >
            Previous
          </button>
          <span className="uppercase tracking-[0.14em] text-neutral-500">
            Page {page} / {pages}
          </span>
          <button
            type="button"
            disabled={page >= pages}
            onClick={() => setQuery({ page: page + 1 })}
            className="rounded-full border border-emerald-200 px-4 py-2 disabled:opacity-40"
          >
            Next
          </button>
        </nav>
      ) : null}
    </main>
  );
}
