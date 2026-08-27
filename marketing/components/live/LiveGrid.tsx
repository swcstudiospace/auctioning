import HoverCard from "./HoverCard";
import styles from "./live.module.css";

export type LiveTone = "emerald" | "pink";

export type LiveStatus = "HOT" | "PHOTO" | "REIGN" | "DARK_HORSE" | "COOLING";

export type LiveRow = {
  position: number;
  domain: string;
  mark: string;
  tone: LiveTone;
  status: LiveStatus;
  raceRp: number;
  gap: number;
  blurb: string;
  footer: string;
  paidPct: number;
  communityPct: number;
  lastOvertake: string;
};

const SPRINT_RP = 20000;

export const LIVE_ROWS: LiveRow[] = [
  {
    position: 1,
    domain: "see.io",
    mark: "S",
    tone: "emerald",
    status: "HOT",
    raceRp: 18420,
    gap: 240,
    blurb: "Creator growth tools for the attention economy",
    footer: "Held P1. Gap 240.",
    paidPct: 94,
    communityPct: 6,
    lastOvertake: "18m",
  },
  {
    position: 2,
    domain: "orynth.dev",
    mark: "O",
    tone: "pink",
    status: "PHOTO",
    raceRp: 18180,
    gap: 90,
    blurb: "Typed runtimes for agent fleets",
    footer: "Held P2. Gap 90.",
    paidPct: 88,
    communityPct: 12,
    lastOvertake: "7m",
  },
  {
    position: 3,
    domain: "crowdply.io",
    mark: "C",
    tone: "emerald",
    status: "REIGN",
    raceRp: 17600,
    gap: 400,
    blurb: "Community boards that actually move",
    footer: "Held P3. Gap 400.",
    paidPct: 81,
    communityPct: 19,
    lastOvertake: "14m",
  },
  {
    position: 4,
    domain: "drynth.dev",
    mark: "D",
    tone: "pink",
    status: "DARK_HORSE",
    raceRp: 16840,
    gap: 220,
    blurb: "Fast catalogs for operators",
    footer: "Held P4. Gap 220.",
    paidPct: 76,
    communityPct: 24,
    lastOvertake: "2m",
  },
  {
    position: 5,
    domain: "latch.tools",
    mark: "L",
    tone: "emerald",
    status: "COOLING",
    raceRp: 15100,
    gap: 800,
    blurb: "Access control for live grids",
    footer: "Held P5. Gap 800.",
    paidPct: 70,
    communityPct: 30,
    lastOvertake: "9m",
  },
];

export default function LiveGrid() {
  return (
    <section className={`ui-card ${styles.grid}`} aria-label="Live grid">
      <div className={styles.colHead} aria-hidden="true">
        <span>Pos</span>
        <span>Competitor</span>
        <span>Race RP</span>
        <span>Progress</span>
        <span>Gap</span>
        <span>Form</span>
      </div>
      <ol className={styles.rows}>
        {LIVE_ROWS.map((row) => (
          <li key={row.domain} className={styles.row} tabIndex={0}>
            <div
              className={styles.rowMain}
              aria-describedby={`hover-${row.position}`}
            >
              <span className={styles.pos}>P{row.position}</span>
              <span className={styles.competitor}>
                <span className={`ui-mark ${styles.mark}`} data-tone={row.tone}>
                  {row.mark}
                </span>
                <span className={styles.identity}>
                  <span className={styles.domain}>{row.domain}</span>
                  <span className={styles.blurb}>{row.blurb}</span>
                </span>
              </span>
              <span className={styles.rp}>
                {row.raceRp.toLocaleString("en-US")}
                <span className={styles.rpUnit}>RP</span>
              </span>
              <span className={styles.progress}>
                <span className={styles.track}>
                  <span
                    className={styles.fill}
                    style={{
                      width: `${Math.max(8, Math.round((row.raceRp / SPRINT_RP) * 100))}%`,
                    }}
                  />
                </span>
              </span>
              <span className={styles.gap}>{row.gap.toLocaleString("en-US")}</span>
              <span className={styles.badge} data-status={row.status}>
                {row.status.replace("_", " ")}
              </span>
            </div>
            <HoverCard row={row} />
          </li>
        ))}
      </ol>
    </section>
  );
}
