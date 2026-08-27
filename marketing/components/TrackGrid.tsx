import Link from "next/link";
import styles from "./TrackGrid.module.css";

const TRACKS = [
  { name: "AI Agents", rp: 48200, mark: "A", accent: "emerald" as const },
  { name: "Developer Tools", rp: 31100, mark: "D", accent: "pink" as const },
  { name: "Creator Tools", rp: 19840, mark: "C", accent: "emerald" as const },
];

const MAX_RP = 48200;

export default function TrackGrid() {
  return (
    <ul className={styles.grid}>
      {TRACKS.map((track) => (
        <li key={track.name}>
          <Link href="/live" className={`ui-card ${styles.card}`}>
            <div className={styles.top}>
              <span
                className={`ui-mark ${styles.mark} ${
                  track.accent === "pink" ? styles.markPink : ""
                }`}
              >
                {track.mark}
              </span>
              <p className={styles.live}>
                <span className={styles.dot} aria-hidden="true" />
                Live
              </p>
            </div>
            <h2>{track.name}</h2>
            <p className={styles.rp}>
              <strong>{track.rp.toLocaleString("en-US")}</strong>
              <span>RP</span>
            </p>
            <div className={styles.bar} aria-hidden="true">
              <span
                className={styles.fill}
                style={{ width: `${(track.rp / MAX_RP) * 100}%` }}
              />
            </div>
          </Link>
        </li>
      ))}
    </ul>
  );
}
