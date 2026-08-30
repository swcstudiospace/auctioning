"use client";

import { useState } from "react";

function hostOf(url: string | null | undefined): string {
  if (!url) return "";
  try {
    return new URL(url).hostname.replace(/^www\./, "");
  } catch {
    return "";
  }
}

export default function CompanyIcon({
  url,
  name,
  size = 40,
}: {
  url?: string | null;
  name: string;
  size?: number;
}) {
  const host = hostOf(url);
  const initial = (name || "?").trim().charAt(0).toUpperCase() || "?";
  const [failed, setFailed] = useState(!host);

  return (
    <span
      className="relative inline-flex shrink-0 items-center justify-center overflow-hidden rounded-full bg-forest text-white"
      style={{ width: size, height: size }}
      aria-hidden
    >
      <span className="text-sm font-bold">{initial}</span>
      {!failed && host ? (
        // eslint-disable-next-line @next/next/no-img-element
        <img
          src={`https://www.google.com/s2/favicons?sz=128&domain=${encodeURIComponent(host)}`}
          alt=""
          width={size}
          height={size}
          className="absolute inset-0 h-full w-full object-cover"
          onError={() => setFailed(true)}
        />
      ) : null}
    </span>
  );
}
