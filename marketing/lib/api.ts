export const API_BASE = process.env.NEXT_PUBLIC_API_URL || "";
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
  kind?: string;
  status?: string;
  starts_at?: string;
  ends_at?: string;
  remaining_secs?: number;
};

export type RaceEvent = {
  kind?: string;
  body?: string;
  created_at?: string;
  attacker?: string;
  victim?: string;
  handle?: string;
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
  if (!API_BASE) return null;
  try {
    const res = await fetch(`${API_BASE}${path}`, { cache: "no-store" });
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
  if (!API_BASE) return { ok: false, status: 0, error: "API URL is not configured" };
  try {
    const res = await fetch(`${API_BASE}${path}`, { cache: "no-store", ...init });
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
      const errObj = parsed as { error?: string } | null;
      return {
        ok: false,
        status: res.status,
        error: errObj?.error || res.statusText || "request failed",
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

export function predictRank(projects: Project[], handle: string, newTotal: number): number {
  const others = projects.filter((p) => p.handle !== handle);
  return 1 + others.filter((p) => p.total_rp > newTotal).length;
}

export function amountToClaimFirst(leaderRp: number, projectRp = 0): number {
  return Math.max(1, leaderRp + 1 - projectRp);
}
