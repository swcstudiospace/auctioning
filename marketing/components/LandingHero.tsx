import Link from "next/link";
import styles from "./LandingHero.module.css";

const TRACKS = [
  { name: "AI Agents", rp: 48200, mark: "A", accent: "emerald" as const },
  { name: "Developer Tools", rp: 31100, mark: "D", accent: "pink" as const },
  { name: "Creator Tools", rp: 19840, mark: "C", accent: "emerald" as const },
];

const MAX_RP = 48200;

export default function LandingHero() {
  return (
    <section className={styles.hero} aria-labelledby="landing-hero-title">
      <div className={styles.copy}>
        <p className={styles.kicker}>Windowed RP · live board</p>
        <h1 id="landing-hero-title">
          The live race
          <br />
          for attention.
        </h1>
        <p className={styles.lede}>
          Rank is windowed RP. News is the product. $1 still buys 1 paid RP.
        </p>
        <Link href="/enter" className={`ui-btn-gradient ${styles.cta}`}>
          Enter the grid
        </Link>
      </div>

      <div className={styles.stack} aria-label="Live track stacks">
        {TRACKS.map((track) => (
          <article key={track.name} className={`ui-card ${styles.card}`}>
            <span
              className={`ui-mark ${styles.mark} ${
                track.accent === "pink" ? styles.markPink : ""
              }`}
            >
              {track.mark}
            </span>
            <div className={styles.meta}>
              <p className={styles.live}>
                <span className={styles.dot} aria-hidden="true" />
                Live
              </p>
              <h2 className={styles.trackName}>{track.name}</h2>
            </div>
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
          </article>
        ))}
      </div>
    </section>
  );
}
