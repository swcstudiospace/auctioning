"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import { MagicCard } from "@/components/magic/MagicCard";
import { ShinyButton } from "@/components/magic/ShinyButton";
import { BlurFade } from "@/components/magic/BlurFade";
import { BorderBeam } from "@/components/magic/BorderBeam";
import { fetchContent, fetchTape, fetchWindows, type ContentItem } from "@/lib/race";
import type { RaceEvent } from "@/lib/api";
import { getJson } from "@/lib/api";
import { allPosts } from "@/lib/blog";

type Recap = {
  id: string;
  kind: string;
  body: string;
  source: string;
};

export default function NewsFeed() {
  const [recaps, setRecaps] = useState<Recap[]>([]);
  const [house, setHouse] = useState<ContentItem[]>([]);
  const [loaded, setLoaded] = useState(false);
  const posts = allPosts();
  const featured = posts[0];

  useEffect(() => {
    (async () => {
      const windows = await fetchWindows();
      const items: Recap[] = [];
      for (const w of windows.slice(0, 6)) {
        if (!w.slug) continue;
        const tape = await fetchTape(w.slug);
        for (const p of tape) {
          items.push({
            id: p.event_id || `${w.slug}-${items.length}`,
            kind: (p.channel || "recap").toUpperCase(),
            body: p.body,
            source: p.source || "template",
          });
        }
        if (!tape.length) {
          const ev = await getJson<{ events?: RaceEvent[] }>(
            `/v1/races/windows/${encodeURIComponent(w.slug)}/events`,
          );
          for (const e of ev?.events || []) {
            const body = e.body || e.summary || e.title || "";
            if (!body) continue;
            items.push({
              id: `${w.slug}-${body}`,
              kind: (e.kind || e.event_type || "event").toUpperCase(),
              body,
              source: "ledger",
            });
          }
        }
      }
      const content = await fetchContent();
      setRecaps(items);
      setHouse(content);
      setLoaded(true);
    })();
  }, []);

  return (
    <main className="mx-auto max-w-6xl px-6 py-10">
      <BlurFade>
        <p className="k text-forest">House journal</p>
        <h1 className="mt-2 max-w-3xl text-4xl font-bold leading-[1.05] md:text-6xl">
          News from the grid.
        </h1>
        <p className="mt-4 max-w-2xl text-neutral-600">
          Posts from the house. Recaps from the ledger. Nothing invented.
        </p>
      </BlurFade>

      {featured ? (
        <BlurFade delay={0.08}>
          <Link href={`/news/${featured.slug}`} className="mt-10 block">
            <article className="relative overflow-hidden rounded-3xl border border-emerald-100 bg-[#0a0a0a] p-8 text-[#EDEAE2] md:p-12">
              <BorderBeam size={100} duration={10} />
              <div className="pointer-events-none absolute inset-0 bg-[radial-gradient(circle_at_top_right,rgba(62,142,98,0.28),transparent_42%)]" />
              <div className="relative grid gap-8 md:grid-cols-[auto_1fr] md:items-end">
                <div className="font-mono text-5xl leading-none text-forest md:text-7xl">
                  {featured.dateLabel.split(" ")[0]}
                  <span className="mt-2 block text-xs tracking-[0.2em] text-white/40">
                    {featured.dateLabel.replace(/^\d+\s/, "").toUpperCase()}
                  </span>
                </div>
                <div>
                  <span className="chip bg-forest/20 text-emerald-200">{featured.kicker}</span>
                  <h2 className="mt-4 text-3xl font-bold md:text-5xl">{featured.title}</h2>
                  <p className="mt-4 max-w-xl text-white/60">{featured.excerpt}</p>
                  <p className="mt-6 text-[11px] tracking-[0.16em] text-white/40">
                    {featured.author} · READ THE LAUNCH →
                  </p>
                </div>
              </div>
            </article>
          </Link>
        </BlurFade>
      ) : null}

      {posts.length > 1 ? (
        <>
          <p className="k mt-14">Archive</p>
          <div className="mt-4 grid gap-4 md:grid-cols-2">
            {posts.slice(1).map((p, i) => (
              <BlurFade key={p.slug} delay={0.04 * i}>
                <Link href={`/news/${p.slug}`} className="block h-full">
                  <MagicCard beam={i === 0} className="h-full">
                    <span className="chip">{p.kicker}</span>
                    <h3 className="mt-4 text-2xl font-bold">{p.title}</h3>
                    <p className="mt-2 text-sm text-neutral-600">{p.excerpt}</p>
                    <p className="mt-6 text-[11px] tracking-[0.16em] text-neutral-500">
                      {p.dateLabel} · {p.author}
                    </p>
                  </MagicCard>
                </Link>
              </BlurFade>
            ))}
          </div>
        </>
      ) : null}

      <p className="k mt-14">How they did it</p>
      {!loaded ? (
        <p className="mt-3 text-sm text-neutral-500">Loading recaps…</p>
      ) : recaps.length === 0 ? (
        <MagicCard className="mt-3">
          <p className="text-sm text-neutral-600">
            No ledger recaps yet. First overtake, photo finish, or archived sprint becomes the first story.
          </p>
        </MagicCard>
      ) : (
        <div className="mt-4 grid gap-4 md:grid-cols-3">
          {recaps.slice(0, 12).map((n) => (
            <MagicCard key={n.id}>
              <span className="chip">{n.kind}</span>
              <p className="mt-3 text-sm leading-6">{n.body}</p>
              <div className="mt-4 border-t border-emerald-100 pt-2 text-[10px] tracking-[0.16em] text-neutral-500">
                {n.source}
              </div>
            </MagicCard>
          ))}
        </div>
      )}

      {house.length ? (
        <>
          <p className="k mt-14">From the tape</p>
          <div className="mt-4 grid gap-4 md:grid-cols-2">
            {house.map((c) => (
              <MagicCard key={c.slug}>
                <h3 className="text-lg font-bold">{c.title}</h3>
                <p className="mt-2 text-sm text-neutral-600">{c.body_md}</p>
              </MagicCard>
            ))}
          </div>
        </>
      ) : null}

      <div className="relative mt-14 overflow-hidden rounded-3xl border border-emerald-100 bg-white p-8">
        <BorderBeam />
        <div className="relative z-[1] flex flex-wrap items-center justify-between gap-4">
          <div>
            <h2 className="text-2xl font-bold">Ready to race?</h2>
            <p className="text-sm text-neutral-600">Claim #1 on the live catalog.</p>
          </div>
          <ShinyButton href="/rank#claim">Claim #1 →</ShinyButton>
        </div>
      </div>
    </main>
  );
}
