import Link from "next/link";
import styles from "./FinalCta.module.css";

const APP_URL = process.env.NEXT_PUBLIC_APP_URL ?? "http://localhost:3000";

export default function FinalCta() {
  return (
    <section className={styles.section} aria-labelledby="final-cta-title">
      <p className={styles.kicker}>Your move</p>
      <h2 id="final-cta-title">Enter the grid</h2>
      <div className={styles.actions}>
        <Link href="/enter" className={`ui-btn-gradient ${styles.primary}`}>
          Enter the grid
        </Link>
        <a href={APP_URL} className={styles.launch}>
          Launch app
        </a>
      </div>
      <p className={styles.legal}>
        $1 = 1 paid RP; community RP is not money.
      </p>
    </section>
  );
}
