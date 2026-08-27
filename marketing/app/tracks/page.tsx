import TrackGrid from "../../components/TrackGrid";
import styles from "../../components/TrackGrid.module.css";

export default function TracksPage() {
  return (
    <main className="ui-page">
      <header className={styles.pageHead}>
        <h1>Tracks</h1>
      </header>
      <TrackGrid />
    </main>
  );
}
