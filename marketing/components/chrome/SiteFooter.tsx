import Link from "next/link";
import { brand } from "../../lib/brand";
import styles from "./SiteFooter.module.css";

export default function SiteFooter() {
  return (
    <footer className={styles.footer}>
      <p className={styles.disclaimer}>
        {brand.name} is a community project. Free and community RP is promotional
        and non-cashable — it is not money and never cashes out. Paid RP is a
        consumable utility ($1 = 1 paid RP) with provenance on Solana. Not
        financial advice.
      </p>
      <nav className={styles.links} aria-label="Legal">
        <Link href="/tos/">Terms</Link>
        <Link href="/privacy/">Privacy</Link>
        <Link href="/legal/">Legal</Link>
      </nav>
    </footer>
  );
}
