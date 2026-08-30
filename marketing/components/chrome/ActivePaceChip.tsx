"use client";

import { useEffect, useState } from "react";
import { getJson } from "@/lib/api";

export type ActivePaceCard = {
  slug?: string;
  name?: string;
  multiplier_bps?: number;
  ends_at?: string;
};

export function ActivePaceChip() {
  const [card, setCard] = useState<ActivePaceCard | null>(null);

  useEffect(() => {
    let cancel = false;
    getJson<{ active?: ActivePaceCard | null }>("/v1/events/active").then((res) => {
      if (cancel || !res?.active?.name) return;
      setCard(res.active);
    });
    return () => {
      cancel = true;
    };
  }, []);

  if (!card?.name) return null;
  const x = card.multiplier_bps ? (card.multiplier_bps / 10000).toFixed(2).replace(/\.00$/, "") : "";
  return (
    <span className="chip bg-forest text-white" title={card.ends_at ? `until ${card.ends_at}` : undefined}>
      {card.name}
      {x ? ` · ${x}× pace` : ""}
    </span>
  );
}
