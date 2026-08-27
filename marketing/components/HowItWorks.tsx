import styles from "./HowItWorks.module.css";

export default function HowItWorks() {
  return (
    <section className={styles.section} aria-labelledby="how-it-works-title">
      <header className={styles.header}>
        <p className={styles.kicker}>Four moving parts</p>
        <h2 id="how-it-works-title">How it works</h2>
      </header>

      <div className={styles.tiles}>
        <article className={`ui-card ${styles.tile}`}>
          <p className={styles.index}>01</p>
          <h3>Fuel</h3>
          <p className={styles.body}>
            Paid RP is a purchase. Community RP is a stipend. Community RP is not
            money — it never cashes out.
          </p>
          <div className={styles.pair}>
            <div>
              <span>Paid</span>
              <strong>$1 = 1 RP</strong>
            </div>
            <div>
              <span>Community</span>
              <strong>Not money</strong>
            </div>
          </div>
        </article>

        <article className={`ui-card ${styles.tile}`}>
          <p className={`${styles.index} ${styles.indexPink}`}>02</p>
          <h3>Grid</h3>
          <p className={styles.body}>
            Rank is windowed RP. The board only counts what landed in the current
            window — yesterday does not linger.
          </p>
          <div className={styles.window} aria-hidden="true">
            {Array.from({ length: 8 }, (_, i) => (
              <span
                key={i}
                className={i >= 5 ? styles.tickOn : styles.tick}
              />
            ))}
          </div>
        </article>

        <article className={`ui-card ${styles.tile}`}>
          <p className={styles.index}>03</p>
          <h3>Speed</h3>
          <p className={styles.body}>
            Pace, velocity, and burst describe how RP arrived. They are derived
            from the window, not extra scores you can buy.
          </p>
          <dl className={styles.metrics}>
            <div>
              <dt>Pace</dt>
              <dd>Derived</dd>
            </div>
            <div>
              <dt>Velocity</dt>
              <dd>Derived</dd>
            </div>
            <div>
              <dt>Burst</dt>
              <dd>Derived</dd>
            </div>
          </dl>
        </article>

        <article className={`ui-card ${styles.tile}`}>
          <p className={`${styles.index} ${styles.indexPink}`}>04</p>
          <h3>Featured</h3>
          <p className={styles.body}>
            Featured is a watchability score. It is not first place, and it is not
            who spent the most.
          </p>
          <p className={styles.pull}>Not who is first.</p>
        </article>
      </div>
    </section>
  );
}
