import Link from "next/link";
import { MagicCard } from "@/components/magic/MagicCard";
import { ShinyButton } from "@/components/magic/ShinyButton";

const PILLARS = [
  ["PAID", "$1 buys 1 paid RP. That ratio is advertised and on-chain."],
  ["COMMUNITY", "50 RP a week, off-chain, non-cashable, expires with the week."],
  ["PACE", "Afterburner and Night Grid add event_multiplier lots. They never land in paid_rp."],
  ["NEWS", "Recaps mint as drafts. Nothing ships until the operator desk approves."],
];

const RULES = [
  { n: "01", t: "Lifetime rank is catalog RP. A race window is a separate board; overtaking in a GP does not rewrite lifetime place." },
  { n: "02", t: "Hover briefing stays in one dock. Rows stay scan-only: position, name, one grid badge, RP, gap." },
  { n: "03", t: "Badges (HOT, REIGN, DARK HORSE, PHOTO, COOLING) come from the race engine, not copy." },
  { n: "04", t: "Championship is points. GP P1–P10 score 25…1. Sprint P1–P3 score 8/7/6. Fastest pace +1." },
  { n: "05", t: "Clicks are first-party. CPC is race RP / board clicks. We do not scrape ARR." },
  { n: "06", t: "Wallet signing is prepare-* plus Phantom. Shuttle never holds the key." },
];

export default function RulesPage() {
  return (
    <main className="mx-auto max-w-6xl px-6 py-10">
      <Link href="/rank" className="chip">
        ← Back to rank
      </Link>
      <p className="k mt-6 text-forest">House playbook</p>
      <h1 className="mt-2 text-4xl font-bold">Race rules</h1>
      <p className="mt-3 max-w-2xl text-neutral-600">
        Play to rank. Fuel is Racing Points. News is what the grid did, after a human says ship it.
      </p>
      <p className="k mt-10">Game pillars</p>
      <div className="mt-3 grid gap-4 md:grid-cols-4">
        {PILLARS.map(([k, p]) => (
          <MagicCard key={k}>
            <span className="chip">{k}</span>
            <p className="mt-3 text-sm">{p}</p>
          </MagicCard>
        ))}
      </div>
      <p className="k mt-10">House rules 01–06</p>
      <div className="mt-3 grid gap-4 md:grid-cols-3">
        {RULES.map((r) => (
          <MagicCard key={r.n} className="flex gap-4">
            <span className="text-4xl font-bold text-emerald-200">{r.n}</span>
            <p className="text-sm">{r.t}</p>
          </MagicCard>
        ))}
      </div>
      <MagicCard className="mt-6">
        <div className="k text-forest">Worked example</div>
        <p className="mt-2 text-lg">
          Buy <b className="text-forest">100 paid RP</b> during Afterburner (1.5×). Ledger writes{" "}
          <b className="text-forest">100 paid</b> + <b className="text-forest">50 pace</b>. Rank ads still say $1 = 1.
        </p>
      </MagicCard>
      <div className="mt-8 flex justify-center">
        <ShinyButton href="/news/launching-auctioning-lol">Read the launch</ShinyButton>
      </div>
    </main>
  );
}
