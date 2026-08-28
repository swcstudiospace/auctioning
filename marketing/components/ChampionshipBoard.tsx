import styles from "./ChampionshipBoard.module.css";

type FormLetter = "G" | "S" | "M" | "P";
type MarkTone = "emerald" | "pink";

type Standing = {
  pos: number;
  company: string;
  points: number;
  wins: number;
  best: string;
  sprint: number;
  letter: string;
  tone: MarkTone;
  form: FormLetter[];
};

const STANDINGS: Standing[] = [
  {
    pos: 1,
    company: "see.io",
    points: 100,
    wins: 2,
    best: "1st",
    sprint: 25,
    letter: "S",
    tone: "emerald",
    form: ["G", "G", "G", "G", "G"],
  },
  {
    pos: 2,
    company: "bidhouse",
    points: 82,
    wins: 1,
    best: "2nd",
    sprint: 18,
    letter: "B",
    tone: "pink",
    form: ["S", "G", "G", "S", "M"],
  },
  {
    pos: 3,
    company: "auctionlab",
    points: 64,
    wins: 0,
    best: "3rd",
    sprint: 12,
    letter: "A",
    tone: "emerald",
    form: ["M", "S", "M", "G", "M"],
  },
  {
    pos: 4,
    company: "wavebid",
    points: 51,
    wins: 0,
    best: "4th",
    sprint: 9,
    letter: "W",
    tone: "pink",
    form: ["S", "M", "S", "M", "S"],
  },
  {
    pos: 5,
    company: "gavelco",
    points: 40,
    wins: 0,
    best: "5th",
    sprint: 6,
    letter: "G",
    tone: "emerald",
    form: ["M", "M", "G", "M", "M"],
  },
  {
    pos: 6,
    company: "hammerly",
    points: 28,
    wins: 0,
    best: "6th",
    sprint: 4,
    letter: "H",
    tone: "pink",
    form: ["P", "M", "S", "M", "P"],
  },
  {
    pos: 7,
    company: "salegrid",
    points: 19,
    wins: 0,
    best: "7th",
    sprint: 2,
    letter: "S",
    tone: "emerald",
    form: ["M", "P", "M", "P", "M"],
  },
  {
    pos: 8,
    company: "xbid",
    points: 12,
    wins: 0,
    best: "8th",
    sprint: 1,
    letter: "X",
    tone: "pink",
    form: ["P", "P", "M", "P", "P"],
  },
];

const FORM_LABEL: Record<FormLetter, string> = {
  G: "Good",
  S: "Strong",
  M: "Mid",
  P: "Poor",
};

const CHIP: Record<FormLetter, string> = {
  G: styles.chipG,
  S: styles.chipS,
  M: styles.chipM,
  P: styles.chipP,
};

export default function ChampionshipBoard() {
  return (
    <section className={styles.board}>
      <header className={styles.head}>
        <h1>2026 Season 1 Championship</h1>
        <p className={styles.season}>
          <span className={styles.dot} aria-hidden="true" />
          2026 Season 1
        </p>
      </header>

      <div className={styles.layout}>
        <div className="ui-card">
          <div className={styles.tableWrap}>
            <table className={styles.table}>
              <caption className={styles.caption}>
                2026 Season 1 Championship standings by points
              </caption>
              <thead>
                <tr>
                  <th scope="col">Pos</th>
                  <th scope="col">Company</th>
                  <th scope="col">Points</th>
                  <th scope="col">Wins</th>
                  <th scope="col">Best</th>
                  <th scope="col">Sprint pts</th>
                </tr>
              </thead>
              <tbody>
                {STANDINGS.map((row) => (
                  <tr
                    key={row.company}
                    className={row.pos === 1 ? styles.p1 : undefined}
                  >
                    <td className={styles.pos}>P{row.pos}</td>
                    <td>
                      <span className={styles.company}>
                        <span
                          className={`ui-mark ${styles.mark}`}
                          data-tone={row.tone}
                        >
                          {row.letter}
                        </span>
                        {row.company}
                      </span>
                    </td>
                    <td className={styles.points}>{row.points}</td>
                    <td>{row.wins}</td>
                    <td>{row.best}</td>
                    <td>{row.sprint}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>

        <aside className={`ui-card ${styles.form}`} aria-labelledby="champ-form-title">
          <h2 id="champ-form-title">Form Guide</h2>
          <p className={styles.formLegend}>
            Last 5 events · G = Good · S = Strong · M = Mid · P = Poor
          </p>
          <ol className={styles.formList}>
            {STANDINGS.map((row) => (
              <li key={row.company}>
                <span className={styles.formWho}>
                  <span className={styles.formPos}>P{row.pos}</span>
                  {row.company}
                </span>
                <span
                  className={styles.formChips}
                  aria-label={`${row.company} form ${row.form
                    .map((letter) => FORM_LABEL[letter])
                    .join(", ")}`}
                >
                  {row.form.map((letter, index) => (
                    <span
                      key={`${row.company}-${index}`}
                      className={`${styles.chip} ${CHIP[letter]}`}
                    >
                      {letter}
                    </span>
                  ))}
                </span>
              </li>
            ))}
          </ol>
          <p className={styles.formNote}>
            Form reflects the last 5 scored events
          </p>
        </aside>
      </div>

      <p className={styles.updated}>
        Data updated May 18, 2026 · 14:30 UTC
      </p>
    </section>
  );
}
