"use client";

export default function Error({
  reset,
}: {
  error: Error & { digest?: string };
  reset: () => void;
}) {
  return (
    <main className="mx-auto max-w-xl px-6 py-16">
      <p className="text-xs font-medium uppercase tracking-[0.18em] text-forest">Unable to load</p>
      <h1 className="mt-3 text-2xl font-semibold text-ink">Unable to load data at this time</h1>
      <p className="mt-3 text-sm text-ink/70">
        The site is up. Retry in a moment. If this persists, the catalog API is not reachable from
        this deploy.
      </p>
      <button
        type="button"
        onClick={() => reset()}
        className="mt-8 rounded-full bg-forest px-5 py-2 text-sm font-medium text-white"
      >
        Retry
      </button>
    </main>
  );
}
