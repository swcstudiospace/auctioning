import styles from "./live.module.css";

const TOUR_DAY = 4;
const TOUR_DAYS = 7;

export default function CalendarRail() {
  return (
    <aside className={styles.rail} aria-label="Race calendar">
      <section className={`ui-card ${styles.railCard}`}>
        <div className={styles.railTop}>
          <p className={styles.railLabel}>Green Flag</p>
          <span className={styles.liveTag}>
            <span className={styles.liveDot} aria-hidden="true" />
            Live
          </span>
        </div>
        <p className={styles.clock}>00:42</p>
      </section>

      <section className={`ui-card ${styles.railCard}`}>
        <div className={styles.railTop}>
          <p className={styles.railLabel}>Grand Tour</p>
          <p className={styles.railLabel}>Day {TOUR_DAY} of {TOUR_DAYS}</p>
        </div>
        <div className={styles.segments} aria-hidden="true">
          {Array.from({ length: TOUR_DAYS }, (_, i) => (
            <span
              key={i}
              className={`${styles.seg} ${i < TOUR_DAY ? styles.segOn : ""}`}
            />
          ))}
        </div>
      </section>

      <section className={`ui-card ${styles.railCard}`}>
        <p className={styles.railLabel}>Championship</p>
        <div className={styles.champ}>
          <span className={`ui-mark ${styles.mark}`} data-tone="emerald">
            S
          </span>
          <span className={styles.identity}>
            <span className={styles.domain}>P1 see.io</span>
          </span>
          <span className={styles.pts}>100 pts</span>
        </div>
      </section>
    </aside>
  );
}
