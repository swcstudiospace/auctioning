"use client";

import { useCallback, useEffect, useState } from "react";
import {
  narrativeDecide,
  narrativeQueue,
  type NarrativeQueueRow,
} from "@/lib/api";

const FILTERS = ["draft", "approved", "skipped", "published", "failed"] as const;

export default function NarrativeDesk() {
  const [status, setStatus] = useState<string>("draft");
  const [rows, setRows] = useState<NarrativeQueueRow[]>([]);
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    setError(null);
    setRows(await narrativeQueue(status));
  }, [status]);

  useEffect(() => {
    void load();
  }, [load]);

  async function act(id: string, action: "approve" | "skip" | "mark-published") {
    setBusy(id + action);
    const result = await narrativeDecide(id, action);
    setBusy(null);
    if (!result.ok) {
      setError(result.error);
      return;
    }
    await load();
  }

  return (
    <main className="mx-auto max-w-3xl px-6 py-10">
      <p className="k text-forest">Operator</p>
      <h1 className="mt-2 text-4xl font-bold">Narrative desk</h1>
      <p className="mt-3 max-w-xl text-sm text-neutral-600">
        Approve or skip recaps. This never posts to X or TikTok — it only records the decision on the ledger.
      </p>

      <div className="mt-6 flex flex-wrap gap-2">
        {FILTERS.map((f) => (
          <button
            key={f}
            type="button"
            onClick={() => setStatus(f)}
            className={"chip" + (status === f ? " bg-forest text-white" : "")}
          >
            {f}
          </button>
        ))}
      </div>

      {error ? <p className="mt-4 text-sm text-red-700">{error}</p> : null}

      <ul className="mt-6 space-y-4">
        {rows.map((row) => (
          <li key={row.id} className="rounded-2xl border border-emerald-100 bg-white p-5">
            <div className="flex flex-wrap items-center justify-between gap-2 text-[11px] uppercase tracking-wide text-neutral-500">
              <span>
                {row.channel} · {row.publish_status}
              </span>
              <span className="font-mono">{new Date(row.created_at).toISOString().slice(0, 16)}Z</span>
            </div>
            <p className="mt-3 whitespace-pre-wrap text-sm leading-relaxed">{row.body}</p>
            {row.last_error ? <p className="mt-2 text-xs text-red-700">{row.last_error}</p> : null}
            <div className="mt-4 flex flex-wrap gap-2">
              {row.publish_status === "draft" || row.publish_status === "failed" ? (
                <button
                  type="button"
                  disabled={busy != null}
                  onClick={() => void act(row.id, "approve")}
                  className="rounded-full bg-forest px-4 py-1.5 text-[11px] font-semibold uppercase tracking-wide text-white disabled:opacity-50"
                >
                  Approve
                </button>
              ) : null}
              {row.publish_status === "draft" || row.publish_status === "approved" ? (
                <button
                  type="button"
                  disabled={busy != null}
                  onClick={() => void act(row.id, "skip")}
                  className="rounded-full border border-emerald-200 px-4 py-1.5 text-[11px] font-semibold uppercase tracking-wide disabled:opacity-50"
                >
                  Skip
                </button>
              ) : null}
              {row.publish_status === "approved" ? (
                <button
                  type="button"
                  disabled={busy != null}
                  onClick={() => void act(row.id, "mark-published")}
                  className="rounded-full border border-forest px-4 py-1.5 text-[11px] font-semibold uppercase tracking-wide text-forest disabled:opacity-50"
                >
                  Mark published
                </button>
              ) : null}
            </div>
          </li>
        ))}
      </ul>

      {!rows.length ? (
        <p className="mt-10 text-sm text-neutral-500">
          No {status} recaps. Race events mint drafts; nothing ships until you approve.
        </p>
      ) : null}
    </main>
  );
}
