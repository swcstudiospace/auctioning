import Link from "next/link";
import CompanyIcon from "@/components/chrome/CompanyIcon";
import { RaceBadge } from "@/components/chrome/RaceBadge";
import { MagicCard } from "@/components/magic/MagicCard";
import type { GridSlot, Project } from "@/lib/api";
import { metricsFromProject, type HoverMetrics } from "@/lib/race";

export type GarageStats = HoverMetrics;

export function statsFromProject(p: Project, slot?: GridSlot | null, gapToNext?: number): GarageStats {
  return metricsFromProject(p, slot, gapToNext != null ? { gapToNext } : undefined);
}

function fmt(n: number): string {
  return n.toLocaleString("en-US");
}

function Metric({ k, v }: { k: string; v: string }) {
  return (
    <div className="rounded-xl border border-emerald-50 bg-white px-3 py-2">
      <div className="k">{k}</div>
      <div className="mt-0.5 font-mono text-sm">{v}</div>
    </div>
  );
}

export default function GarageCard({
  stats,
  compact = false,
}: {
  stats: GarageStats;
  compact?: boolean;
}) {
  const href = `/garage/${encodeURIComponent(stats.handle)}`;

  if (compact) {
    return (
      <div className="rounded-2xl border border-emerald-100 bg-white p-4 shadow-lg">
        <p className="k">Identity</p>
        <div className="mt-1 flex items-start justify-between gap-3">
          <div className="flex min-w-0 items-center gap-3">
            <CompanyIcon url={stats.url} name={stats.displayName} handle={stats.handle} size={36} />
            <div className="min-w-0">
              <h2 className="truncate text-base font-bold">{stats.displayName}</h2>
              {stats.tag ? (
                <p className="mt-0.5 text-[11px] uppercase tracking-wide text-neutral-500">
                  {stats.tag.replace(/-/g, " ")}
                </p>
              ) : null}
            </div>
          </div>
          {stats.badge ? <RaceBadge badge={stats.badge} /> : null}
        </div>
        <p className="k mt-3">Racing</p>
        <div className="mt-1 grid grid-cols-5 gap-1">
          <Metric k="POS" v={`P${stats.position}`} />
          <Metric k="GAP" v={fmt(stats.gap)} />
          <Metric k="PACE" v={`${stats.pace}`} />
          <Metric k="VEL" v={`${stats.velocity}`} />
          <Metric k="PASS" v={stats.lastOvertake || "—"} />
        </div>
        <p className="k mt-3">Intel</p>
        <div className="mt-1 grid grid-cols-5 gap-1">
          <Metric k="RACE" v={fmt(stats.raceRp)} />
          <Metric k="LIFE" v={fmt(stats.lifetimeRp)} />
          <Metric k="PAID" v={`${stats.paidPct}%`} />
          <Metric k="CLICKS" v={fmt(stats.clicks)} />
          <Metric k="CPC" v={stats.cpc == null ? "—" : stats.cpc.toFixed(1)} />
        </div>
        <p className="mt-3 text-xs text-neutral-600">{stats.footer}</p>
        <Link href={href} className="mt-2 inline-block text-xs font-semibold uppercase tracking-wide text-forest">
          Open garage →
        </Link>
      </div>
    );
  }

  return (
    <div>
      <div className="flex items-start justify-between gap-3">
        <div className="flex min-w-0 items-start gap-3">
          <CompanyIcon url={stats.url} name={stats.displayName} handle={stats.handle} size={48} />
          <div className="min-w-0">
            <h2 className="text-4xl font-bold">{stats.displayName} GARAGE</h2>
            {stats.blurb ? <p className="mt-2 max-w-xl text-neutral-600">{stats.blurb}</p> : null}
          </div>
        </div>
        <RaceBadge badge={stats.badge} />
      </div>
      <div className="mt-4 grid gap-2 md:grid-cols-5">
        <MagicCard><div className="k">POSITION</div><div className="mt-1 font-mono text-3xl">P{stats.position}</div></MagicCard>
        <MagicCard><div className="k">GAP</div><div className="mt-1 font-mono text-3xl">{fmt(stats.gap)}</div></MagicCard>
        <MagicCard><div className="k">PACE</div><div className="mt-1 font-mono text-3xl">{stats.pace}</div></MagicCard>
        <MagicCard><div className="k">VELOCITY</div><div className="mt-1 font-mono text-3xl">{stats.velocity}</div></MagicCard>
        <MagicCard><div className="k">LAST OVERTAKE</div><div className="mt-1 font-mono text-lg">{stats.lastOvertake || "—"}</div></MagicCard>
      </div>
      <div className="mt-2 grid gap-2 md:grid-cols-5">
        <MagicCard><div className="k">RACE RP</div><div className="mt-1 font-mono text-3xl">{fmt(stats.raceRp)}</div></MagicCard>
        <MagicCard><div className="k">LIFETIME RP</div><div className="mt-1 font-mono text-3xl">{fmt(stats.lifetimeRp)}</div></MagicCard>
        <MagicCard>
          <div className="k">PAID / COMMUNITY</div>
          <div className="mt-1 font-mono text-3xl">{stats.paidPct}%</div>
          <div className="mt-2 h-2 overflow-hidden rounded-full bg-neutral-200">
            <div className="h-full bg-forest" style={{ width: `${Math.min(100, Math.max(0, stats.paidPct))}%` }} />
          </div>
        </MagicCard>
        <MagicCard><div className="k">CLICKS</div><div className="mt-1 font-mono text-3xl">{fmt(stats.clicks)}</div></MagicCard>
        <MagicCard><div className="k">CPC</div><div className="mt-1 font-mono text-3xl">{stats.cpc == null ? "—" : stats.cpc.toFixed(1)}</div></MagicCard>
      </div>
      <p className="mt-3 text-sm text-neutral-600">{stats.footer}</p>
    </div>
  );
}
