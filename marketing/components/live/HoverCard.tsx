import styles from "./live.module.css";
import type { LiveRow } from "./LiveGrid";

export default function HoverCard({ row }: { row: LiveRow }) {
  return (
    <aside
      className={styles.hoverCard}
      id={`hover-${row.position}`}
      aria-label={`${row.domain} race card`}
    >
      <div className={styles.hoverHead}>
        <span className={`ui-mark ${styles.mark}`} data-tone={row.tone}>
          {row.mark}
        </span>
        <span className={styles.identity}>
          <span className={styles.domain}>{row.domain}</span>
          <span className={styles.blurb}>{row.blurb}</span>
        </span>
        <span className={styles.liveTag}>
          <span className={styles.liveDot} aria-hidden="true" />
          Live
        </span>
      </div>

      <div className={styles.statRow}>
        <dl className={styles.stat}>
          <dt>Place</dt>
          <dd>P{row.position}</dd>
        </dl>
        <dl className={styles.stat}>
          <dt>Gap</dt>
          <dd>{row.gap.toLocaleString("en-US")}</dd>
        </dl>
        <dl className={styles.stat}>
          <dt>Last overtake</dt>
          <dd>{row.lastOvertake}</dd>
        </dl>
      </div>

      <div className={styles.biRow}>
        <dl className={styles.bi}>
          <dt>Race RP</dt>
          <dd>{row.raceRp.toLocaleString("en-US")}</dd>
        </dl>
        <dl className={styles.bi}>
          <dt>Lifetime</dt>
          <dd>P{row.position}</dd>
        </dl>
        <dl className={styles.bi}>
          <dt>Paid</dt>
          <dd>{row.paidPct}%</dd>
        </dl>
        <dl className={styles.bi}>
          <dt>Community</dt>
          <dd>{row.communityPct}%</dd>
        </dl>
        <dl className={styles.bi}>
          <dt>Clicks</dt>
          <dd>—</dd>
        </dl>
        <dl className={styles.bi}>
          <dt>CPC</dt>
          <dd>—</dd>
        </dl>
      </div>

      <div
        className={styles.mix}
        aria-hidden="true"
      >
        <span className={styles.mixPaid} style={{ width: `${row.paidPct}%` }} />
        <span
          className={styles.mixCommunity}
          style={{ width: `${row.communityPct}%` }}
        />
      </div>
      <p className={styles.caption}>
        board clicks, unverified. Paid RP is $1 = 1. Community RP is not money.
      </p>
      <p className={styles.hoverFooter}>{row.footer}</p>
    </aside>
  );
}
