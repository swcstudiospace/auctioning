function trimBase(raw: string | undefined): string {
  return (raw || "").trim().replace(/\/$/, "");
}

/**
 * Browser: NEXT_PUBLIC_API_URL, or same-origin (Next rewrites /v1 → API).
 * Server: AUCTIONING_INTERNAL_API_URL, then public URL, then local runner.
 */
export function resolveApiBase(): string {
  const pub = trimBase(process.env.NEXT_PUBLIC_API_URL);
  if (typeof window === "undefined") {
    return trimBase(process.env.AUCTIONING_INTERNAL_API_URL) || pub || "http://127.0.0.1:8000";
  }
  return pub;
}

export const API_BASE = resolveApiBase();
export const WHOP_CHECKOUT_URL = process.env.NEXT_PUBLIC_WHOP_CHECKOUT_URL || "";

export type GridSlot = {
  handle: string;
  rank: number;
  race_rp: number;
  velocity?: number;
  gap_to_leader?: number;
  gap_to_next?: number | null;
  paid_rp?: number;
  community_rp?: number;
  badge?: string | null;
  last_overtake?: string | null;
  clicks?: number;
  hover_footer?: string;
  owner?: string;
};

export type RaceWindow = {
  slug: string;
  name?: string;
  kind?: string;
  race_type?: string;
  status?: string;
  starts_at?: string;
  ends_at?: string;
  remaining_secs?: number;
  tag?: string;
};

export type RaceEvent = {
  kind?: string;
  event_type?: string;
  title?: string;
  summary?: string;
  body?: string;
  created_at?: string;
  attacker?: string;
  victim?: string;
  handle?: string;
  project_handle?: string;
};

export type Project = {
  handle: string;
  owner_wallet: string | null;
  source: string;
  source_ref: string | null;
  display_name: string | null;
  blurb: string | null;
  stable_id: string | null;
  url: string | null;
  tags: string[];
  total_rp: number;
  rank: number;
  clicks?: number;
};

export type ProjectList = {
  projects: Project[];
  total: number;
  page: number;
  per_page: number;
  tags?: string[];
};

export type RpView = {
  wallet: string;
  paid_rp: number;
  free_rp: number;
  spent_rp: number;
  free_rp_non_cashable?: boolean;
};

export type SupportOutcome = {
  project_total_rp: number;
  from_free: number;
  from_paid: number;
};

export type ApiResult<T> =
  | { ok: true; data: T }
  | { ok: false; status: number; error: string };

export async function getJson<T>(path: string): Promise<T | null> {
  try {
    const res = await fetch(`${resolveApiBase()}${path}`, { cache: "no-store" });
    if (!res.ok) return null;
    return (await res.json()) as T;
  } catch {
    return null;
  }
}

export async function postJson<T>(path: string, body: unknown): Promise<T | null> {
  const result = await apiFetch<T>(path, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(body),
  });
  return result.ok ? result.data : null;
}

export async function apiFetch<T>(path: string, init?: RequestInit): Promise<ApiResult<T>> {
  try {
    const res = await fetch(`${resolveApiBase()}${path}`, { cache: "no-store", ...init });
    const text = await res.text();
    let parsed: unknown = null;
    if (text) {
      try {
        parsed = JSON.parse(text);
      } catch {
        parsed = { error: text };
      }
    }
    if (!res.ok) {
      const errObj = parsed as { error?: string; message?: string } | null;
      return {
        ok: false,
        status: res.status,
        error: errObj?.message || errObj?.error || res.statusText || "request failed",
      };
    }
    return { ok: true, data: parsed as T };
  } catch (e) {
    return { ok: false, status: 0, error: e instanceof Error ? e.message : "network error" };
  }
}

export async function listProjects(params: {
  page?: number;
  per_page?: number;
  tag?: string;
  q?: string;
}): Promise<ApiResult<ProjectList>> {
  const qs = new URLSearchParams();
  qs.set("page", String(params.page ?? 1));
  qs.set("per_page", String(params.per_page ?? 50));
  if (params.tag) qs.set("tag", params.tag);
  if (params.q) qs.set("q", params.q);
  return apiFetch<ProjectList>(`/v1/projects?${qs.toString()}`);
}


export async function submitProject(body: {
  url: string;
  display_name?: string;
  blurb?: string;
  owner_wallet?: string;
}): Promise<ApiResult<{ created: boolean; project: Project }>> {
  return apiFetch("/v1/projects", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(body),
  });
}

export async function getRp(wallet: string): Promise<ApiResult<RpView>> {
  return apiFetch<RpView>(`/v1/rp/${encodeURIComponent(wallet)}`);
}

export async function claimWeekly(wallet: string): Promise<ApiResult<{ claimed?: boolean; amount?: number }>> {
  return apiFetch(`/v1/rp/claim-weekly`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ wallet }),
  });
}

export async function supportProject(
  handle: string,
  body: { wallet: string; amount: number; reason?: string },
): Promise<ApiResult<SupportOutcome>> {
  return apiFetch(`/v1/projects/${encodeURIComponent(handle)}/support`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(body),
  });
}

export async function recordClick(handle: string): Promise<void> {
  await apiFetch(`/v1/projects/${encodeURIComponent(handle)}/click`, { method: "POST" });
}

export type NarrativeQueueRow = {
  id: string;
  event_id: string;
  channel: string;
  body: string;
  publish_status: string;
  external_post_id?: string | null;
  last_error?: string | null;
  retryable?: boolean;
  created_at: string;
};

export async function narrativeQueue(status?: string): Promise<NarrativeQueueRow[]> {
  const qs = status ? `?status=${encodeURIComponent(status)}` : "";
  const rows = await getJson<NarrativeQueueRow[]>(`/v1/narrative/queue${qs}`);
  return Array.isArray(rows) ? rows : [];
}

export async function narrativeDecide(
  id: string,
  action: "approve" | "skip" | "mark-published",
  extra?: { external_post_id?: string },
): Promise<ApiResult<NarrativeQueueRow>> {
  const path = `/v1/narrative/posts/${encodeURIComponent(id)}/${action}`;
  if (action === "mark-published") {
    return apiFetch(path, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(extra || {}),
    });
  }
  return apiFetch(path, { method: "POST" });
}

export function predictRank(projects: Project[], handle: string, newTotal: number): number {
  const others = projects.filter((p) => p.handle !== handle);
  return 1 + others.filter((p) => p.total_rp > newTotal).length;
}

export function amountToClaimFirst(leaderRp: number, projectRp = 0): number {
  return Math.max(1, leaderRp + 1 - projectRp);
}
