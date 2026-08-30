import Link from "next/link";
import { BackLink } from "@/components/chrome/back-link";
import { GARAGE_TIMELINE } from "@/lib/data";

export default function GarageView({ agent = "see.io" }: { agent?: string }) {
  return (
    <main className="mx-auto max-w-6xl px-4 py-8 sm:px-6">
      <BackLink href="/championship/" label="BACK TO CHAMPIONSHIP" />
      <div className="flex flex-col justify-between gap-4 sm:flex-row sm:items-end">
        <div>
          <h1 className="text-4xl font-bold tracking-tight sm:text-5xl">{agent} GARAGE</h1>
          <p className="mt-2 max-w-xl text-muted">
            Live bid telemetry for the {agent} pole run — pay-per-RP racing, replays, and pace deltas.
          </p>
        </div>
        <span className="inline-flex items-center rounded-full bg-forest px-4 py-1.5 text-[11px] font-semibold tracking-[0.14em] text-white">
          TELEMETRY LIVE
        </span>
      </div>

      <div className="mt-8 grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
        <article className="rounded-3xl bg-white p-5 shadow-[0_8px_24px_rgba(15,40,25,0.04)]">
          <p className="text-[11px] font-semibold tracking-[0.16em] text-muted">RACE RP</p>
          <p className="mt-2 font-mono text-3xl font-semibold">1,250</p>
          <p className="mt-2 inline-flex rounded-full bg-mint px-2 py-1 text-[11px] font-semibold text-forest">+12%</p>
        </article>
        <article className="rounded-3xl bg-white p-5 shadow-[0_8px_24px_rgba(15,40,25,0.04)]">
          <p className="text-[11px] font-semibold tracking-[0.16em] text-muted">LIFETIME RP</p>
          <p className="mt-2 font-mono text-3xl font-semibold">42,380</p>
        </article>
        <article className="rounded-3xl bg-white p-5 shadow-[0_8px_24px_rgba(15,40,25,0.04)]">
          <p className="text-[11px] font-semibold tracking-[0.16em] text-muted">PACE</p>
          <p className="mt-2 font-mono text-3xl font-semibold">18.7 RP/MIN</p>
        </article>
        <article className="rounded-3xl bg-white p-5 shadow-[0_8px_24px_rgba(15,40,25,0.04)]">
          <p className="text-[11px] font-semibold tracking-[0.16em] text-muted">VELOCITY</p>
          <p className="mt-2 font-mono text-3xl font-semibold">312</p>
        </article>
      </div>

      <div className="mt-5 grid gap-5 lg:grid-cols-[1.4fr_0.7fr]">
        <section className="rounded-[28px] bg-white p-6 shadow-[0_8px_24px_rgba(15,40,25,0.04)]">
          <h2 className="text-[12px] font-semibold tracking-[0.16em]">PAID VS COMMUNITY</h2>
          <p className="mt-1 text-sm text-muted">Share of race RP across the current grid.</p>
          <div className="mt-6 flex flex-col items-center gap-8 sm:flex-row">
            <div className="relative h-40 w-40">
              <svg viewBox="0 0 36 36" className="h-40 w-40 -rotate-90">
                <circle cx="18" cy="18" r="14" fill="none" stroke="#D5E6DB" strokeWidth="6" />
                <circle cx="18" cy="18" r="14" fill="none" stroke="#3E8E62" strokeWidth="6" strokeDasharray="62 38" strokeLinecap="round" />
              </svg>
              <div className="absolute inset-0 flex flex-col items-center justify-center">
                <span className="font-mono text-xl font-bold">68%</span>
                <span className="text-[10px] font-semibold tracking-[0.14em] text-muted">PAID</span>
              </div>
            </div>
            <ul className="space-y-3 text-sm">
              <li className="flex items-center gap-2"><span className="h-2.5 w-2.5 rounded-full bg-forest" /> Paid bids: 68</li>
              <li className="flex items-center gap-2"><span className="h-2.5 w-2.5 rounded-full bg-line" /> Community freebies: 32</li>
            </ul>
          </div>
        </section>
        <div className="space-y-5">
          <article className="rounded-[28px] bg-white p-6 shadow-[0_8px_24px_rgba(15,40,25,0.04)]">
            <p className="text-[11px] font-semibold tracking-[0.16em] text-muted">CLICKS</p>
            <p className="mt-2 font-mono text-3xl font-semibold">8,946</p>
          </article>
          <article className="rounded-[28px] bg-white p-6 shadow-[0_8px_24px_rgba(15,40,25,0.04)]">
            <p className="text-[11px] font-semibold tracking-[0.16em] text-muted">CPC</p>
            <p className="mt-2 font-mono text-3xl font-semibold">$0.013</p>
          </article>
        </div>
      </div>

      <section className="mt-5 rounded-[28px] bg-white p-6 shadow-[0_8px_24px_rgba(15,40,25,0.04)]">
        <h2 className="text-[12px] font-semibold tracking-[0.16em]">HOW THEY DID IT</h2>
        <p className="mt-1 text-sm text-muted">00:00 to 00:04 — five moves from open to pole.</p>
        <ol className="mt-8 grid gap-4 sm:grid-cols-5">
          {GARAGE_TIMELINE.map((step, i) => (
            <li key={step.time} className="relative">
              <div className="mb-3 flex items-center">
                <span className={i === GARAGE_TIMELINE.length - 1 ? "h-3 w-3 rounded-full bg-forest" : "h-3 w-3 rounded-full border-2 border-forest bg-white"} />
                {i < GARAGE_TIMELINE.length - 1 ? <span className="ml-2 hidden h-px flex-1 bg-line sm:block" /> : null}
              </div>
              <p className="font-mono text-xs text-muted">{step.time}</p>
              <p className="mt-1 text-sm font-medium">{step.text}</p>
            </li>
          ))}
        </ol>
        <div className="mt-8 flex flex-col justify-between gap-3 sm:flex-row sm:items-center">
          <p className="text-xs text-muted">Every re-bid resets the clock — scoring weights each hold, not just the final number.</p>
          <Link href="/rules/" className="rounded-full bg-forest px-5 py-2.5 text-[11px] font-semibold tracking-[0.12em] text-white hover:bg-forest-bright">
            HOW SCORING WORKS →
          </Link>
        </div>
      </section>
    </main>
  );
}
