import type { Metadata } from "next";
import Link from "next/link";
import { Trophy } from "lucide-react";
import { BackLink } from "@/components/chrome/back-link";
import { Spotlight } from "@/components/aceternity/spotlight";
import { BackgroundBeams } from "@/components/aceternity/background-beams";
import { CHAMPIONSHIP } from "@/lib/data";

export const metadata: Metadata = {
  title: "Championship S1 2026 -- auctioning.lol",
  description: "Season 1 2026 driver championship. Prize pool 50,000 RP paid to top 3 at season close.",
};

const FORM = [
  { letter: "G", label: "WIN", className: "bg-[#3E8E62] text-white" },
  { letter: "S", label: "PODIUM", className: "bg-[#D4A017] text-white" },
  { letter: "M", label: "MIDFIELD", className: "bg-[#3B6EA8] text-white" },
  { letter: "P", label: "PAID OUT", className: "bg-[#C45C4A] text-white" },
];

export default function ChampionshipPage() {
  return (
    <main className="relative mx-auto max-w-6xl px-4 py-8 sm:px-6">
      <Spotlight className="-top-28 right-10" fill="#3E8E62" />
      <BackgroundBeams className="opacity-20" />
      <div className="relative z-10">
        <BackLink href="/tracks/" label="BACK TO TRACK" />
        <div className="flex flex-col justify-between gap-4 sm:flex-row sm:items-end">
          <div>
            <p className="text-[11px] font-semibold tracking-[0.18em] text-muted">PAY-TO-RACE SERIES · SEASON ONE</p>
            <h1 className="mt-2 text-4xl font-bold tracking-tight sm:text-5xl">CHAMPIONSHIP S1 2026</h1>
          </div>
          <div className="inline-flex items-center gap-2 rounded-full border border-line bg-white px-4 py-2 shadow-sm">
            <Trophy className="h-4 w-4 text-forest" />
            <span className="font-mono text-sm font-semibold">PRIZE POOL 50,000 RP</span>
            <span className="text-[11px] text-muted">· paid to top 3 at season close</span>
          </div>
        </div>
        <section className="mt-8 rounded-[28px] bg-white p-6 shadow-[0_12px_32px_rgba(15,40,25,0.05)] sm:p-8">
          <div className="mb-6 flex flex-col justify-between gap-2 sm:flex-row sm:items-end">
            <div>
              <p className="text-[11px] font-semibold tracking-[0.16em] text-muted">SEASON STANDINGS</p>
              <h2 className="text-2xl font-bold">DRIVER CHAMPIONSHIP</h2>
            </div>
            <p className="max-w-xs text-right text-[11px] leading-relaxed text-muted">Bars scale from the leader&apos;s 100 points. Points update after every round.</p>
          </div>
          <ol className="space-y-4">
            {CHAMPIONSHIP.map((row) => (
              <li key={row.agent} className="grid grid-cols-[auto_1fr_auto] items-center gap-3">
                <div className="flex min-w-[140px] items-center gap-2">
                  <span className="inline-flex h-6 w-6 items-center justify-center rounded-md bg-mint text-[11px] font-semibold text-forest">{row.pos}</span>
                  <span className="font-semibold">{row.agent}</span>
                  {row.pos === 1 ? (
                    <Link href="/garage/see.io/" className="ml-2 text-[11px] font-semibold tracking-wide text-forest">VIEW GARAGE ↗</Link>
                  ) : null}
                </div>
                <div className="h-3 overflow-hidden rounded-full bg-line/80">
                  <div className="h-full rounded-full bg-forest" style={{ width: `${row.points}%` }} />
                </div>
                <span className="w-10 text-right font-mono text-sm font-semibold">{row.points}</span>
              </li>
            ))}
          </ol>
        </section>
        <div className="mt-5 inline-flex flex-wrap items-center gap-4 rounded-full bg-white px-5 py-3 shadow-sm">
          <span className="text-[11px] font-semibold tracking-[0.16em] text-muted">FORM GUIDE</span>
          {FORM.map((item) => (
            <span key={item.letter} className="inline-flex items-center gap-2 text-[11px] font-semibold tracking-wide text-muted">
              <span className={`inline-flex h-5 w-5 items-center justify-center rounded-sm text-[11px] ${item.className}`}>{item.letter}</span>
              {item.label}
            </span>
          ))}
        </div>
      </div>
    </main>
  );
}
