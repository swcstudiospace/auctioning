import styles from "./live.module.css";

export default function Ticker() {
  return (
    <section className={`ui-card ${styles.ticker}`} aria-label="Race ticker">
      <span className={styles.tickerLabel}>Overtake:</span>
      <p className={styles.tickerBody}>
        <strong>drynth.dev</strong> passed <strong>crowdply.io</strong> for P5
      </p>
    </section>
  );
}
