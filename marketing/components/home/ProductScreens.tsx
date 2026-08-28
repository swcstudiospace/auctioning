import Link from "next/link";
import ChampionshipBoard from "../ChampionshipBoard";
import EnterRace from "../EnterRace";
import LiveGrid from "../live/LiveGrid";
import styles from "./ProductScreens.module.css";

export default function ProductScreens() {
  return (
    <section className={styles.section} aria-labelledby="product-screens-title">
      <header className={styles.header}>
        <p className={styles.kicker}>The boards</p>
        <h2 id="product-screens-title">How the race actually looks</h2>
      </header>

      <ol className={styles.shots}>
        <li className={styles.shot}>
          <div className={styles.meta}>
            <p className={styles.shotKicker}>Live</p>
            <h3 className={styles.shotTitle}>The live board</h3>
            <Link href="/live" className={styles.open}>
              Open live
            </Link>
          </div>
          <figure className={styles.frame}>
            <div className={styles.chrome} aria-hidden="true">
              <span className={styles.dots}>
                <span className={styles.traffic} />
                <span className={styles.traffic} />
                <span className={styles.traffic} />
              </span>
              <span className={styles.address}>auctioning.lol /live</span>
            </div>
            <div className={`${styles.viewport} ${styles.viewportLive}`} inert>
              <LiveGrid />
            </div>
          </figure>
        </li>

        <li className={styles.shot}>
          <div className={styles.meta}>
            <p className={styles.shotKicker}>Championship</p>
            <h3 className={styles.shotTitle}>Season standings</h3>
            <Link href="/championship" className={styles.open}>
              Open championship
            </Link>
          </div>
          <figure className={styles.frame}>
            <div className={styles.chrome} aria-hidden="true">
              <span className={styles.dots}>
                <span className={styles.traffic} />
                <span className={styles.traffic} />
                <span className={styles.traffic} />
              </span>
              <span className={styles.address}>auctioning.lol /championship</span>
            </div>
            <div className={`${styles.viewport} ${styles.viewportChamp}`} inert>
              <ChampionshipBoard />
            </div>
          </figure>
        </li>

        <li className={styles.shot}>
          <div className={styles.meta}>
            <p className={styles.shotKicker}>Enter</p>
            <h3 className={styles.shotTitle}>Add paid RP</h3>
            <Link href="/enter" className={styles.open}>
              Open enter
            </Link>
          </div>
          <figure className={styles.frame}>
            <div className={styles.chrome} aria-hidden="true">
              <span className={styles.dots}>
                <span className={styles.traffic} />
                <span className={styles.traffic} />
                <span className={styles.traffic} />
              </span>
              <span className={styles.address}>auctioning.lol /enter</span>
            </div>
            <div className={`${styles.viewport} ${styles.viewportEnter}`} inert>
              <EnterRace />
            </div>
          </figure>
        </li>
      </ol>
    </section>
  );
}
