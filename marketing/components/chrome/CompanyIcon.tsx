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

function hostFromHandle(handle: string): string {
  const token = (handle || "").trim().split(/[\s·/_]+/)[0].replace(/_/g, ".");
  if (token.includes(".") && /^[a-z0-9.-]+$/i.test(token)) return token;
  return "";
}

function sourcesFor(host: string): string[] {
  const h = encodeURIComponent(host);
  return [
    `https://www.google.com/s2/favicons?sz=128&domain=${h}`,
    `https://icons.duckduckgo.com/ip3/${host}.ico`,
    `https://${host}/favicon.ico`,
  ];
}

export default function CompanyIcon({
  url,
  name,
  handle,
  size = 40,
}: {
  url?: string | null;
  name: string;
  handle?: string;
  size?: number;
}) {
  const host = hostOf(url) || hostFromHandle(handle || "") || hostFromHandle(name);
  const sources = host ? sourcesFor(host) : [];
  const initial = (name || "?").trim().charAt(0).toUpperCase() || "?";
  const [idx, setIdx] = useState(0);
  const src = sources[idx];

  return (
    <span
      className="relative inline-flex shrink-0 items-center justify-center overflow-hidden rounded-full bg-forest text-white"
      style={{ width: size, height: size }}
      aria-hidden
    >
      <span className="text-sm font-bold">{initial}</span>
      {src ? (
        // eslint-disable-next-line @next/next/no-img-element
        <img
          src={src}
          alt=""
          width={size}
          height={size}
          className="absolute inset-0 h-full w-full object-cover"
          onError={() => setIdx((i) => i + 1)}
        />
      ) : null}
    </span>
  );
}
