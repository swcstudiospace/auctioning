import type { Metadata } from "next";
import Link from "next/link";
import CalendarRail from "../../components/live/CalendarRail";
import LiveGrid from "../../components/live/LiveGrid";
import Ticker from "../../components/live/Ticker";
import styles from "../../components/live/live.module.css";

export const metadata: Metadata = {
  title: "Live Grid — 2026 Season 1 — auctioning.lol",
  description:
    "Live race grid for auctioning.lol 2026 Season 1. Paid RP is $1 = 1. Community RP is not money.",
};

export default function LivePage() {
  return (
    <main className={`ui-page ${styles.page}`}>
      <header className={styles.seasonHead}>
        <p className={styles.kicker}>2026 Season 1</p>
        <h1>Live Grid</h1>
      </header>

      <nav className={styles.tabs} aria-label="Season boards">
        <Link className={`${styles.tab} ${styles.tabActive}`} href="/live/" aria-current="page">
          Live Grid
        </Link>
        <Link className={styles.tab} href="/championship/">
          Championship
        </Link>
        <Link className={styles.tab} href="/news/">
          News
        </Link>
      </nav>

      <div className={styles.board}>
        <LiveGrid />
        <CalendarRail />
      </div>
      <Ticker />
    </main>
  );
}
