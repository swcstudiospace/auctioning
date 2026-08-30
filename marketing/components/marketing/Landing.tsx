import Link from "next/link";
import { BackgroundBeams } from "@/components/aceternity/BackgroundBeams";
import { Spotlight } from "@/components/aceternity/Spotlight";
import { Marquee } from "@/components/magic/Marquee";
import { ShinyButton } from "@/components/magic/ShinyButton";
import { MagicCard } from "@/components/magic/MagicCard";
import { BlurFade } from "@/components/magic/BlurFade";
import { NumberTicker } from "@/components/magic/NumberTicker";
import { Meteors } from "@/components/magic/Meteors";
import { AnimatedShinyText } from "@/components/magic/AnimatedShinyText";
import { BorderBeam } from "@/components/magic/BorderBeam";
import { Logo } from "@/components/chrome/Logo";

const features = [
  { k: "PUBLIC", h: "Rank is the scoreboard", p: "The catalog is open. You do not log in to see who is #1." },
  { k: "50 RP", h: "Weekly community fuel", p: "Connect Phantom. Claim 50 RP every week. Vote. It is not money and it does not cash out." },
  { k: "PAID", h: "$1 = 1 RP on-chain", p: "Paid fuel is logged through Phantom. Community RP stays in Postgres." },
  { k: "GRID", h: "The catalog starts at 0", p: "Seeded from outbid.lol. Nobody inherited rank. First fuel writes the first position." },
  { k: "GARAGE", h: "Hover is the briefing", p: "Ten metrics on hover. The company page is telemetry, not a top-level tab." },
  { k: "POINTS", h: "Championship is points", p: "GP P1–P10, sprint P1–P3, fastest pace +1. Empty until a window archives." },
  { k: "LIVE", h: "One race at a time", p: "Calendar rail for sprint, GP, and whoever is actually leading." },
  { k: "NEWS", h: "News is the product", p: "House blog plus ledger recaps. No invented ROI." },
];

const surfaces = [
  { href: "/rank", k: "RANK", h: "The catalog", p: "Search, page, hover, fuel." },
  { href: "/tracks", k: "TRACK", h: "Sector scrap", p: "One tag, one board. Wins from archived sprints." },
  { href: "/live", k: "LIVE", h: "F1 weekend", p: "Sprint, Grand Prix, championship rail. One grid." },
  { href: "/championship", k: "CHAMPIONSHIP", h: "Points table", p: "Not catalog RP." },
  { href: "/news", k: "NEWS", h: "The house blog", p: "Launch post is up. Recaps wait on the desk." },
  { href: "/rules", k: "RULES", h: "House playbook", p: "$1 = 1. Weekly 50. Pace is extra." },
  { href: "/rank#add", k: "ADD", h: "List your site", p: "URL in. 0 RP. Fuel to climb." },
];

const faqs = [
  {
    q: "Do I have to log in to use the site?",
    a: "No. Rank, news, live, championship, and rules are public. Phantom is only to claim weekly RP, vote, or pay for fuel.",
  },
  {
    q: "What is the 50 RP per week?",
    a: "Community fuel. One claim a week after you connect. Use it to vote the grid. It is not cash and it does not withdraw.",
  },
  {
    q: "Is paid RP the same thing?",
    a: "No. Paid RP is $1 = 1 RP, signed on-chain. Community RP is a weekly allocation. Different ledgers.",
  },
  {
    q: "Why is the catalog already full?",
    a: "The first grid is the outbid.lol catalog, imported at 0 RP. The names are real. The rank is not seeded.",
  },
  {
    q: "Where are the races?",
    a: "Live race is the F1 weekend: one grid, a sprint countdown, a 7-day Grand Prix, and a championship points table. Rank is lifetime. Live is windowed RP.",
  },
  {
    q: "Where did garage go?",
    a: "It was never a destination. Open a listing. Hover is the briefing. /garage/[handle] is the company page.",
  },
];

export default function Landing({ catalogTotal = 0 }: { catalogTotal?: number }) {
  const n = catalogTotal > 0 ? catalogTotal : 0;
  const tape = [
    "RP is fuel",
    "News is the product",
    "Rank is the scoreboard",
    "50 RP / week",
    "Phantom",
    "Solana",
    n ? `${n.toLocaleString()} companies` : "Open catalog",
    "0 inherited RP",
  ];
  return (
    <div className="bg-[#0a0a0a] text-[#EDEAE2]">
      <Spotlight className="overflow-hidden">
        <BackgroundBeams />
        <Meteors number={18} />
        <div className="relative mx-auto max-w-6xl px-6 pb-20 pt-16 md:pt-24">
          <BlurFade>
            <p className="k">
              <AnimatedShinyText className="tracking-[0.18em] uppercase text-[11px]">
                Play to rank
              </AnimatedShinyText>
            </p>
            <h1 className="mt-4 max-w-3xl text-4xl font-bold leading-[1.05] md:text-6xl">
              Fuel the board.
              <br />
              Own the rank.
            </h1>
            <p className="mt-6 max-w-xl text-base text-white/60 md:text-lg">
              Business racing. RP is fuel. Rank is the scoreboard. News is the product.{" "}
              {n ? (
                <>
                  <NumberTicker value={n} /> companies.{" "}
                </>
              ) : null}
              Zero inherited RP. 50 RP every week to vote.
            </p>
          </BlurFade>
          <BlurFade delay={0.12}>
            <div className="mt-8 flex flex-wrap items-center gap-3">
              <ShinyButton href="/rank">Enter the grid</ShinyButton>
              <Link
                href="/live"
                className="inline-flex items-center rounded-full border border-white/20 px-5 py-2.5 text-sm font-semibold uppercase tracking-wide hover:border-forest"
              >
                Watch live
              </Link>
              <Link
                href="/enter"
                className="inline-flex items-center rounded-full border border-white/20 px-5 py-2.5 text-sm font-semibold uppercase tracking-wide hover:border-forest"
              >
                Claim 50 RP
              </Link>
            </div>
            <p className="mt-4 text-xs tracking-[0.16em] text-white/40">
              RANK IS PUBLIC · PHANTOM IS ONLY FOR FUEL
            </p>
          </BlurFade>
        </div>
      </Spotlight>

      <div className="mx-auto max-w-6xl space-y-2 px-6">
        <Marquee items={tape} />
        <Marquee items={[...tape].reverse()} reverse className="opacity-70" />
      </div>

      <section id="how" className="mx-auto max-w-6xl px-6 py-20">
        <BlurFade>
          <p className="k text-forest">The OS, for racing</p>
          <h2 className="mt-2 text-3xl font-bold md:text-4xl">What you actually get.</h2>
          <p className="mt-3 max-w-2xl text-white/55">
            Same shape as a product OS: surfaces, not a pile of landing claims. Login is a door to fuel, not the site.
          </p>
        </BlurFade>
        <div className="mt-10 grid gap-4 md:grid-cols-4">
          {features.map((f, i) => (
            <BlurFade key={f.k} delay={0.04 * i}>
              <MagicCard tone="dark" className="h-full">
                <span className="chip">{f.k}</span>
                <h3 className="mt-3 text-lg font-semibold">{f.h}</h3>
                <p className="mt-2 text-sm text-white/55">{f.p}</p>
              </MagicCard>
            </BlurFade>
          ))}
        </div>
      </section>

      <section id="surfaces" className="mx-auto max-w-6xl px-6 py-10">
        <BlurFade>
          <p className="k text-forest">Product</p>
          <h2 className="mt-2 text-3xl font-bold md:text-4xl">The grid, in motion.</h2>
        </BlurFade>
        <div className="mt-8 grid gap-4 md:grid-cols-3">
          {surfaces.map((s, i) => (
            <BlurFade key={s.k} delay={0.05 * i}>
              <Link href={s.href} className="block h-full">
                <MagicCard tone="dark" beam={s.k === "RANK"} className="h-full">
                  <span className="chip">{s.k}</span>
                  <h3 className="mt-3 text-xl font-bold">{s.h}</h3>
                  <p className="mt-2 text-sm text-white/55">{s.p}</p>
                </MagicCard>
              </Link>
            </BlurFade>
          ))}
        </div>
      </section>

      <section className="mx-auto max-w-6xl px-6 py-16">
        <BlurFade>
          <MagicCard tone="dark" beam className="grid gap-8 p-8 md:grid-cols-2">
            <div>
              <p className="k text-forest">Fuel</p>
              <h2 className="mt-2 text-3xl font-bold">50 RP a week. Then you vote.</h2>
              <p className="mt-3 text-white/55">
                Community RP is how the grid stays alive without turning the homepage into a paywall.
                Connect once. Claim once a week. Put it on a company.
              </p>
            </div>
            <div className="space-y-4 text-sm">
              <div className="border-b border-white/10 pb-4">
                <p className="font-semibold">Community</p>
                <p className="mt-1 text-white/55">50 RP / week. Phantom required. Not cashable.</p>
              </div>
              <div className="border-b border-white/10 pb-4">
                <p className="font-semibold">Paid</p>
                <p className="mt-1 text-white/55">$1 = 1 RP. Signed on-chain. Optional.</p>
              </div>
              <div>
                <p className="font-semibold">Board</p>
                <p className="mt-1 text-white/55">Always public. Empty stats stay blank until the ledger writes.</p>
              </div>
            </div>
          </MagicCard>
        </BlurFade>
      </section>

      <section id="faq" className="mx-auto max-w-6xl px-6 py-10">
        <BlurFade>
          <p className="k text-forest">FAQ</p>
          <h2 className="mt-2 text-3xl font-bold">If it is not here, enter the grid.</h2>
        </BlurFade>
        <div className="mt-8 space-y-3">
          {faqs.map((f, i) => (
            <BlurFade key={f.q} delay={0.03 * i}>
              <details className="card-dark p-5">
                <summary className="cursor-pointer font-semibold">{f.q}</summary>
                <p className="mt-3 text-sm text-white/55">{f.a}</p>
              </details>
            </BlurFade>
          ))}
        </div>
      </section>

      <section className="mx-auto max-w-6xl px-6 py-16">
        <div className="relative overflow-hidden rounded-2xl border border-white/10 bg-[#111] p-8">
          <BorderBeam size={90} duration={9} />
          <div className="relative z-[1] flex flex-col items-start justify-between gap-6 md:flex-row md:items-center">
            <div>
              <h2 className="text-3xl font-bold">Subscribe to the grid.</h2>
              <p className="mt-2 text-white/55">Enter public. Claim 50 RP when you want to vote.</p>
            </div>
            <div className="flex flex-wrap gap-3">
              <ShinyButton href="/rank">Enter the grid</ShinyButton>
              <Link
                href="/live"
                className="inline-flex items-center rounded-full border border-white/20 px-5 py-2.5 text-sm font-semibold uppercase tracking-wide hover:border-forest"
              >
                Watch live
              </Link>
              <Link
                href="/enter"
                className="inline-flex items-center rounded-full border border-white/20 px-5 py-2.5 text-sm font-semibold uppercase tracking-wide hover:border-forest"
              >
                Claim 50 RP
              </Link>
            </div>
          </div>
        </div>
      </section>

      <footer className="border-t border-white/10">
        <div className="mx-auto flex max-w-6xl flex-col gap-6 px-6 py-10">
          <div className="flex flex-wrap items-center justify-between gap-4">
            <Logo dark />
            <div className="flex flex-wrap gap-4 text-xs tracking-[0.14em] text-white/40">
              <Link href="/rank">RANK</Link>
              <Link href="/news">NEWS</Link>
              <Link href="/rules">RULES</Link>
              <Link href="/news/launching-auctioning-lol">LAUNCH</Link>
              <Link href="/legal">LEGAL</Link>
            </div>
          </div>
          <p className="text-xs text-white/35">© 2026 auctioning.lol — rank is fueled with RP, never USD.</p>
        </div>
      </footer>
    </div>
  );
}
