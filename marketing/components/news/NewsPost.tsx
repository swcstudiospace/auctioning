import Link from "next/link";
import { MagicCard } from "@/components/magic/MagicCard";
import { ShinyButton } from "@/components/magic/ShinyButton";
import { BorderBeam } from "@/components/magic/BorderBeam";
import { TracingBeam } from "@/components/aceternity/TracingBeam";
import { getPost, allPosts } from "@/lib/blog";
import { Markdown } from "@/lib/markdown";
import { notFound } from "next/navigation";

export default function NewsPost({ slug }: { slug: string }) {
  const post = getPost(slug);
  if (!post) notFound();
  const others = allPosts().filter((p) => p.slug !== slug).slice(0, 2);
  const initials = post.author
    .split(" ")
    .map((w) => w[0])
    .join("")
    .slice(0, 2);

  return (
    <main className="pb-16">
      <header className="relative overflow-hidden border-b border-emerald-100 bg-[#0a0a0a] text-[#EDEAE2]">
        <div className="pointer-events-none absolute inset-0 bg-[radial-gradient(circle_at_20%_0%,rgba(62,142,98,0.35),transparent_50%)]" />
        <div className="relative mx-auto max-w-3xl px-6 py-14 md:py-20">
          <Link href="/news" className="text-[11px] tracking-[0.18em] text-white/45 hover:text-white">
            ← ALL POSTS
          </Link>
          <p className="mt-8 text-[11px] tracking-[0.22em] text-emerald-300">{post.kicker}</p>
          <h1 className="mt-3 text-4xl font-bold leading-[1.05] md:text-6xl">{post.title}</h1>
          <div className="mt-8 flex items-center gap-3">
            <span className="grid h-10 w-10 place-items-center rounded-full bg-forest text-xs font-semibold">
              {initials}
            </span>
            <div>
              <p className="text-sm font-semibold">{post.author}</p>
              <p className="text-[11px] tracking-[0.14em] text-white/40">{post.dateLabel}</p>
            </div>
          </div>
        </div>
      </header>

      <article className="mx-auto max-w-3xl px-6 py-12">
        <p className="text-xl leading-8 text-neutral-600">{post.excerpt}</p>
        <TracingBeam className="mt-12">
          <Markdown source={post.body} />
        </TracingBeam>
      </article>

      {others.length ? (
        <section className="mx-auto max-w-3xl px-6">
          <p className="k">More from the house</p>
          <div className="mt-4 grid gap-4 md:grid-cols-2">
            {others.map((p) => (
              <Link key={p.slug} href={`/news/${p.slug}`}>
                <MagicCard>
                  <span className="chip">{p.kicker}</span>
                  <h3 className="mt-3 text-lg font-bold">{p.title}</h3>
                </MagicCard>
              </Link>
            ))}
          </div>
        </section>
      ) : null}

      <div className="mx-auto mt-12 max-w-3xl px-6">
        <div className="relative overflow-hidden rounded-3xl border border-emerald-100 bg-white p-8">
          <BorderBeam />
          <div className="relative z-[1] flex flex-wrap items-center justify-between gap-4">
            <div>
              <h2 className="text-xl font-bold">Fuel the board</h2>
              <p className="text-sm text-neutral-600">Nobody inherited rank. Claim it.</p>
            </div>
            <ShinyButton href="/rank#claim">Claim #1 →</ShinyButton>
          </div>
        </div>
      </div>
    </main>
  );
}
