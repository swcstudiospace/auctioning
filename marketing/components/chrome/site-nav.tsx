"use client";

import { useEffect, useId, useState } from "react";
import Link from "next/link";
import { usePathname } from "next/navigation";
import { Menu, X } from "lucide-react";
import { NAV } from "@/lib/data";
import { cn } from "@/lib/utils";
import { SiteLogo } from "./logo";

function isActive(pathname: string, href: string) {
  if (href === "/garage/") return pathname.startsWith("/garage");
  return pathname === href || pathname === href.slice(0, -1);
}

export default function SiteNav() {
  const [open, setOpen] = useState(false);
  const pathname = usePathname();
  const drawerId = useId();

  useEffect(() => { setOpen(false); }, [pathname]);

  useEffect(() => {
    if (!open) return;
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") setOpen(false);
    };
    document.addEventListener("keydown", onKey);
    const previous = document.body.style.overflow;
    document.body.style.overflow = "hidden";
    return () => {
      document.removeEventListener("keydown", onKey);
      document.body.style.overflow = previous;
    };
  }, [open]);

  return (
    <header className="sticky top-0 z-50 border-b border-line/70 bg-mint/85 backdrop-blur-md">
      <div className="mx-auto flex h-[72px] max-w-6xl items-center justify-between gap-4 px-4 sm:px-6">
        <SiteLogo />
        <nav className="hidden items-center gap-6 lg:flex" aria-label="Primary">
          {NAV.map((item) => (
            <Link
              key={item.href}
              href={item.href}
              className={cn(
                "text-[12px] font-semibold tracking-[0.14em] text-ink/70 transition hover:text-ink",
                isActive(pathname, item.href) && "text-ink underline decoration-forest decoration-2 underline-offset-8"
              )}
            >
              {item.label}
            </Link>
          ))}
        </nav>
        <Link
          href="/enter/"
          className="hidden rounded-full bg-forest px-5 py-2 text-[12px] font-semibold tracking-[0.12em] text-white shadow-sm transition hover:bg-forest-bright lg:inline-flex"
        >
          PLACE A BID
        </Link>
        <button
          type="button"
          className="inline-flex h-10 w-10 items-center justify-center rounded-full border border-line bg-white text-ink lg:hidden"
          aria-expanded={open}
          aria-controls={drawerId}
          aria-label={open ? "Close menu" : "Open menu"}
          onClick={() => setOpen((v) => !v)}
        >
          {open ? <X className="h-4 w-4" /> : <Menu className="h-4 w-4" />}
        </button>
      </div>
      {open ? (
        <nav id={drawerId} className="flex flex-col gap-1 border-t border-line bg-mint px-4 pb-5 pt-3 lg:hidden" aria-label="Mobile">
          {NAV.map((item) => (
            <Link key={item.href} href={item.href} className="rounded-xl px-3 py-2 text-sm font-semibold tracking-wide text-ink">
              {item.label}
            </Link>
          ))}
          <Link href="/enter/" className="mt-2 rounded-full bg-forest px-5 py-2.5 text-center text-[12px] font-semibold tracking-[0.12em] text-white">
            PLACE A BID
          </Link>
        </nav>
      ) : null}
    </header>
  );
}
