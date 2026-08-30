const LABELS: Record<string, string> = {
  HOT: "HOT",
  REIGN: "REIGN",
  DARK_HORSE: "DARK HORSE",
  PHOTO: "PHOTO",
  COOLING: "COOLING",
};

export function normalizeBadge(raw?: string | null): string | null {
  if (!raw) return null;
  const key = raw.trim().replace(/\s+/g, "_").toUpperCase();
  return LABELS[key] ? key : null;
}

export function RaceBadge({ badge }: { badge?: string | null }) {
  const key = normalizeBadge(badge);
  if (!key) return null;
  return <span className="chip shrink-0">{LABELS[key]}</span>;
}
