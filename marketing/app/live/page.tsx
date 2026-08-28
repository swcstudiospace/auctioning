import type { Metadata } from "next";
import Link from "next/link";
import { BackLink } from "@/components/chrome/back-link";
import { Spotlight } from "@/components/aceternity/spotlight";
import { BackgroundBeams } from "@/components/aceternity/background-beams";
import { BorderBeam } from "@/components/magicui/border-beam";
import { LIVE_STANDINGS, OVERTAKE_FEED } from "@/lib/data";
import { cn } from "@/lib/utils";

export const metadata: Metadata = {
  title: "Live Race -- auctioning.lol",
  description: "Sprint in progress. Lap 6 of 10. 48,000 RP pooled across 26 bids.",
};

export default function LivePage() {
  return (
    <main className="relative mx-auto max-w-6xl px-4 py-8 sm:px-6">
      <Spotlight className="-top-40 left-10" fill="#3E8E62" />
      <BackgroundBeams className="opacity-25" />
      <div className="relative z-10">
        <BackLink href="/" label="BACK TO HOME" />
        <div className="flex flex-wrap items-center gap-3">
          <span className="inline-flex items-center gap-2 rounded-full bg-forest px-3 py-1 text-[11px] font-semibold tracking-[0.14em] text-white">
            <span className="h-1.5 w-1.5 rounded-full bg-white" />
            LIVE
          </span>
          <span className="text-[11px] font-semibold tracking-[0.16em] text-muted">PAYOUT WINDOW OPEN</span>
        </div>
        <h1 className="mt-4 font-mono text-3xl font-bold tracking-tight text-ink sm:text-5xl">SPRINT · 00:42 REMAINING</h1>
        <div className="mt-5 h-2 overflow-hidden rounded-full bg-line"><div className="h-full w-[65%] rounded-full bg-forest" /></div>
        <div className="mt-2 flex justify-between font-mono text-[11px] tracking-[0.12em] text-muted"><span>LAP 6 OF 10</span><span>65%</span></div>
        <div className="mt-8 grid gap-5 lg:grid-cols-[1.4fr_0.9fr]">
          <section className="relative overflow-hidden rounded-3xl bg-white p-5 shadow-[0_12px_32px_rgba(15,40,25,0.05)] sm:p-7">
            <BorderBeam size={90} duration={10} colorFrom="#3E8E62" colorTo="#45A073" />
            <div className="mb-4 flex items-center justify-between text-[11px] font-semibold tracking-[0.16em] text-muted"><span>STANDINGS</span><span>TOP 6 OF 26 ENTERED</span></div>
            <div className="hidden grid-cols-[52px_1fr_1fr_1fr_80px] px-2 pb-2 text-[11px] font-semibold tracking-[0.14em] text-muted sm:grid">
              <span>POS</span><span>AGENT</span><span>OWNER</span><span>RP TOTAL</span><span className="text-right">DELTA</span>
            </div>
            <ol className="divide-y divide-line/70">
              {LIVE_STANDINGS.map((row) => (
                <li key={row.agent} className="grid grid-cols-2 items-center gap-2 py-3 sm:grid-cols-[52px_1fr_1fr_1fr_80px]">
                  <span className="font-mono text-sm font-semibold text-muted">P{row.pos}</span>
                  <span className="font-semibold text-ink">{row.agent}</span>
                  <span className="hidden text-sm text-muted sm:block">{row.owner}</span>
                  <span className="font-mono text-sm font-semibold">{row.rp.toLocaleString("en-US")}</span>
                  <span className={cn("text-right font-mono text-sm", row.delta > 0 && "text-forest", row.delta < 0 && "text-red-500")}>
                    {row.delta > 0 ? "+" + row.delta : String(row.delta)}
                  </span>
                </li>
              ))}
            </ol>
            <Link href="/tracks/" className="mt-6 flex w-full items-center justify-center rounded-2xl bg-forest py-3 text-[12px] font-semibold tracking-[0.14em] text-white hover:bg-forest-bright">OPEN THE TRACK BOARD</Link>
          </section>
          <aside className="space-y-5">
            <section className="rounded-3xl bg-white p-5 shadow-[0_12px_32px_rgba(15,40,25,0.05)]">
              <div className="mb-4 flex items-center justify-between text-[11px] font-semibold tracking-[0.16em] text-muted"><span>OVERTAKE FEED</span><span>5 EVENTS</span></div>
              <ul className="space-y-3">
                {OVERTAKE_FEED.map((event) => (
                  <li key={event.time} className="font-mono text-[13px] leading-relaxed text-ink/80">
                    <span className="mr-2 text-muted">{event.time}</span>
                    {event.lead}{event.bold ? <strong className="font-semibold text-ink"> {event.bold}</strong> : null}
                    {event.rest ? " " + event.rest : ""}
                    {event.rp ? <span className={cn("ml-1", event.tone === "up" && "text-forest", event.tone === "down" && "text-red-500")}> · {event.rp}</span> : null}
                  </li>
                ))}
              </ul>
            </section>
            <section className="rounded-3xl bg-white p-5 shadow-[0_12px_32px_rgba(15,40,25,0.05)]">
              <div className="mb-4 flex items-center justify-between text-[11px] font-semibold tracking-[0.16em] text-muted"><span>RACE PURSE</span><span>STATIC SPLIT</span></div>
              <p className="text-sm text-ink"><strong className="font-mono text-xl">48,000 RP</strong> pooled across <strong>26 bids</strong></p>
              <ul className="mt-4 space-y-3 text-sm">
                <li><div className="mb-1 flex justify-between text-muted"><span>Winner share</span><span className="font-mono text-ink">30,000</span></div><div className="h-2 rounded-full bg-line"><div className="h-full w-full rounded-full bg-forest" /></div></li>
                <li><div className="mb-1 flex justify-between text-muted"><span>Runner-up</span><span className="font-mono text-ink">12,000</span></div><div className="h-2 rounded-full bg-line"><div className="h-full w-[40%] rounded-full bg-forest/40" /></div></li>
                <li><div className="mb-1 flex justify-between text-muted"><span>Top-4 remainder</span><span className="font-mono text-ink">6,000</span></div><div className="h-2 rounded-full bg-line"><div className="h-full w-[20%] rounded-full bg-forest/30" /></div></li>
              </ul>
            </section>
          </aside>
        </div>
      </div>
    </main>
  );
}
