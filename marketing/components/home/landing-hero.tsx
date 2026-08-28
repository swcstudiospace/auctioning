"use client";

import Link from "next/link";
import { Spotlight } from "@/components/aceternity/spotlight";
import { BackgroundBeams } from "@/components/aceternity/background-beams";
import { ShinyButton } from "@/components/magicui/shiny-button";
import { AnimatedShinyText } from "@/components/magicui/animated-shiny-text";
import { BorderBeam } from "@/components/magicui/border-beam";

export default function LandingHero() {
  return (
    <section className="relative overflow-hidden rounded-[28px] border border-line/70 bg-white/70 px-6 py-16 shadow-[0_12px_40px_rgba(15,40,25,0.05)] sm:px-10 sm:py-20">
      <Spotlight className="-top-28 left-0 md:-top-20 md:left-24" fill="#3E8E62" />
      <BackgroundBeams className="opacity-40" />
      <div className="relative z-10 mx-auto max-w-3xl text-center">
        <div className="mb-6 inline-flex rounded-full border border-black/5 bg-mint px-3 py-1">
          <AnimatedShinyText className="text-[11px] font-semibold tracking-[0.18em] text-forest">
            $1 = 1 paid RP · community RP is promotional
          </AnimatedShinyText>
        </div>
        <h1 className="text-4xl font-bold tracking-tight text-ink sm:text-6xl sm:leading-[1.05]">
          Pay to race.
          <br />
          Rank becomes news.
        </h1>
        <p className="mx-auto mt-5 max-w-xl text-lg text-muted">
          Business attention, raced live.
        </p>
        <div className="mt-8 flex flex-col items-center justify-center gap-3 sm:flex-row">
          <Link href="/enter/">
            <ShinyButton className="rounded-full border-forest bg-forest px-8 py-3 text-white shadow-md">
              Place a bid
            </ShinyButton>
          </Link>
          <Link
            href="/live/"
            className="rounded-full border border-line bg-white px-6 py-3 text-sm font-semibold tracking-wide text-ink transition hover:border-forest/40"
          >
            Watch the live race
          </Link>
        </div>
      </div>
      <div className="relative z-10 mx-auto mt-12 max-w-2xl overflow-hidden rounded-2xl border border-line bg-white p-5">
        <BorderBeam size={80} duration={8} colorFrom="#3E8E62" colorTo="#45A073" />
        <p className="text-[11px] font-semibold tracking-[0.16em] text-muted">LIVE SPRINT</p>
        <p className="mt-2 font-mono text-2xl font-semibold text-ink">00:42 remaining · lap 6/10</p>
        <p className="mt-1 text-sm text-muted">see.io leads bidhaus by 578 RP. Purse 48,000 RP across 26 bids.</p>
      </div>
    </section>
  );
}
