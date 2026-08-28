import { Suspense } from "react";
import Leaderboard from "@/components/board/Leaderboard";
import { listProjects, type ProjectList } from "@/lib/api";

export const dynamic = "force-dynamic";

async function loadBoard(sp: { page?: string; tag?: string; q?: string }): Promise<ProjectList | null> {
  const page = Math.max(1, Number(sp.page || "1") || 1);
  const tag = (sp.tag || "").trim();
  const q = (sp.q || "").trim();
  const res = await listProjects({ page, per_page: 50, tag: tag || undefined, q: q || undefined });
  return res.ok ? res.data : null;
}

export default async function LivePage({
  searchParams,
}: {
  searchParams: Promise<{ page?: string; tag?: string; q?: string }>;
}) {
  const sp = await searchParams;
  const initial = await loadBoard(sp);
  return (
    <Suspense fallback={<main className="mx-auto max-w-6xl px-6 py-16 text-neutral-500">Loading board...</main>}>
      <Leaderboard initial={initial} />
    </Suspense>
  );
}
