import Link from "next/link";
import { MagicCard } from "@/components/magic/MagicCard";
import { ShinyButton } from "@/components/magic/ShinyButton";
import { rules } from "@/lib/data";

export default function RulesPage() {
  return (
    <main className="mx-auto max-w-6xl px-6 py-10">
      <Link href="/rank" className="chip">← Back to rank</Link>
      <p className="mt-6 k text-forest">HOUSE PLAYBOOK</p>
      <h1 className="mt-2 text-4xl font-bold">Race Rules</h1>
      <p className="mt-3 max-w-2xl text-neutral-600">
        Every race runs on the same house mechanics: auction the grid, settle every overtake on the clock, pay everything out at the flag.
      </p>
      <p className="k mt-10">GAME PILLARS</p>
      <div className="mt-3 grid gap-4 md:grid-cols-4">
        {[
          ["FUEL", "Racing Points buy grid time."],
          ["GRID", "Six slots P1–P6, settled by highest standing bid."],
          ["SPEED", "Overtakes settle every 60 seconds."],
          ["FEATURED", "Every race lands on the front page."],
        ].map(([k, p]) => (
          <MagicCard key={k}>
            <span className="chip">{k}</span>
            <p className="mt-3 text-sm">{p}</p>
          </MagicCard>
        ))}
      </div>
      <p className="k mt-10">HOUSE RULES 01–06</p>
      <div className="mt-3 grid gap-4 md:grid-cols-3">
        {rules.map((r) => (
          <MagicCard key={r.n} className="flex gap-4">
            <span className="text-4xl font-bold text-emerald-200">{r.n}</span>
            <p className="text-sm">{r.t}</p>
          </MagicCard>
        ))}
      </div>
      <MagicCard className="mt-6">
        <div className="k text-forest">WORKED EXAMPLE</div>
        <p className="mt-2 text-lg">
          Pole costs <b className="text-forest">410 RP</b> · losers split refunds <b className="text-forest">270 RP</b> · net burn holding P3 to flag = <b className="text-forest">140 RP</b>
        </p>
      </MagicCard>
      <div className="mt-8 flex justify-center">
        <ShinyButton href="/news/launching-auctioning-lol">Read the launch</ShinyButton>
      </div>
    </main>
  );
}
