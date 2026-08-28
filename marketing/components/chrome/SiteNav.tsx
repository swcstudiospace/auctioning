"use client";

import { useEffect, useId, useState } from "react";
import Link from "next/link";
import { usePathname } from "next/navigation";
import { brand } from "../../lib/brand";
import styles from "./SiteNav.module.css";

const PRIMARY = [
  { href: "/live/", label: "Live" },
  { href: "/tracks/", label: "Tracks" },
  { href: "/championship/", label: "Championship" },
  { href: "/news/", label: "News" },
  { href: "/rules/", label: "Rules" },
] as const;

export default function SiteNav() {
  const [open, setOpen] = useState(false);
  const pathname = usePathname();
  const drawerId = useId();

  useEffect(() => {
    setOpen(false);
  }, [pathname]);

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

  const close = () => setOpen(false);

  return (
    <header className={styles.bar}>
      <div className={styles.inner}>
        <Link href="/" className={styles.brand}>
          auctioning<span>.lol</span>
        </Link>

        <nav className={styles.desktop} aria-label="Primary">
          <div className={styles.links}>
            {PRIMARY.map((item) => (
              <Link key={item.href} href={item.href} className={styles.link}>
                {item.label}
              </Link>
            ))}
          </div>
          <Link href="/enter/" className={`ui-btn-gradient ${styles.cta}`}>
            Enter Race
          </Link>
          <a href={brand.appUrl} className={styles.launch}>
            Launch app
          </a>
        </nav>

        <button
          type="button"
          className={styles.menuBtn}
          aria-expanded={open}
          aria-controls={drawerId}
          aria-label={open ? "Close menu" : "Open menu"}
          onClick={() => setOpen((value) => !value)}
        >
          <span className={open ? styles.iconOpen : styles.icon} aria-hidden="true" />
        </button>
      </div>

      <div
        className={open ? `${styles.backdrop} ${styles.backdropOpen}` : styles.backdrop}
        onClick={close}
        hidden={!open}
      />

      <nav
        id={drawerId}
        className={open ? `${styles.drawer} ${styles.drawerOpen}` : styles.drawer}
        aria-label="Mobile"
        aria-hidden={!open}
      >
        {PRIMARY.map((item) => (
          <Link key={item.href} href={item.href} className={styles.drawerLink} onClick={close}>
            {item.label}
          </Link>
        ))}
        <Link href="/enter/" className={`ui-btn-gradient ${styles.drawerCta}`} onClick={close}>
          Enter Race
        </Link>
        <a href={brand.appUrl} className={styles.drawerLaunch} onClick={close}>
          Launch app
        </a>
      </nav>
    </header>
  );
}
