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

export default function ChampionshipBoard() {
  return (
    <section className="champ-board">
      <header className="champ-head">
        <h1>2026 Season 1 Championship</h1>
        <p className="champ-season">
          <span className="champ-dot" aria-hidden="true" />
          2026 Season 1
        </p>
      </header>

      <div className="champ-layout">
        <div className="ui-card champ-table-card">
          <div className="champ-table-wrap">
            <table className="champ-table">
              <caption className="champ-caption">
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
                    className={row.pos === 1 ? "champ-p1" : undefined}
                  >
                    <td className="champ-pos">P{row.pos}</td>
                    <td>
                      <span className="champ-company">
                        <span
                          className="ui-mark"
                          data-tone={row.tone}
                          style={{
                            background:
                              row.tone === "pink"
                                ? "var(--pink)"
                                : "var(--emerald)",
                          }}
                        >
                          {row.letter}
                        </span>
                        {row.company}
                      </span>
                    </td>
                    <td className="champ-points">{row.points}</td>
                    <td>{row.wins}</td>
                    <td>{row.best}</td>
                    <td>{row.sprint}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>

        <aside className="ui-card champ-form" aria-labelledby="champ-form-title">
          <h2 id="champ-form-title">Form Guide</h2>
          <p className="champ-form-legend">
            Last 5 events · G = Good · S = Strong · M = Mid · P = Poor
          </p>
          <ol className="champ-form-list">
            {STANDINGS.map((row) => (
              <li key={row.company}>
                <span className="champ-form-who">
                  <span className="champ-form-pos">P{row.pos}</span>
                  {row.company}
                </span>
                <span
                  className="champ-form-chips"
                  aria-label={`${row.company} form ${row.form
                    .map((letter) => FORM_LABEL[letter])
                    .join(", ")}`}
                >
                  {row.form.map((letter, index) => (
                    <span
                      key={`${row.company}-${index}`}
                      className={`champ-chip champ-chip-${letter}`}
                    >
                      {letter}
                    </span>
                  ))}
                </span>
              </li>
            ))}
          </ol>
          <p className="champ-form-note">
            Form reflects the last 5 scored events
          </p>
        </aside>
      </div>

      <p className="champ-updated">
        Data updated May 18, 2026 · 14:30 UTC
      </p>

      <style>{`
        .champ-board {
          max-width: 1080px;
          margin: 0 auto;
          padding: 32px 0 48px;
          color: var(--ink);
        }
        .champ-head {
          display: flex;
          align-items: baseline;
          justify-content: space-between;
          gap: 16px;
          margin-bottom: 24px;
        }
        .champ-head h1 {
          margin: 0;
          font-size: clamp(1.5rem, 3vw, 2rem);
          line-height: 1.2;
          letter-spacing: -0.03em;
          font-weight: 700;
        }
        .champ-season {
          display: flex;
          align-items: center;
          gap: 8px;
          margin: 0;
          color: var(--muted);
          font-size: 13px;
          white-space: nowrap;
        }
        .champ-dot {
          width: 8px;
          height: 8px;
          border-radius: 50%;
          background: var(--emerald);
        }
        .champ-layout {
          display: grid;
          grid-template-columns: minmax(0, 1fr) 280px;
          gap: 24px;
          align-items: start;
        }
        .champ-table-wrap {
          overflow-x: auto;
        }
        .champ-caption {
          position: absolute;
          width: 1px;
          height: 1px;
          padding: 0;
          margin: -1px;
          overflow: hidden;
          clip: rect(0, 0, 0, 0);
          white-space: nowrap;
          border: 0;
        }
        .champ-table {
          width: 100%;
          border-collapse: separate;
          border-spacing: 0 6px;
          font-variant-numeric: tabular-nums;
        }
        .champ-table th {
          text-align: left;
          color: var(--muted);
          font-weight: 500;
          font-size: 12px;
          letter-spacing: 0.04em;
          text-transform: uppercase;
          padding: 0 12px 8px;
        }
        .champ-table td {
          padding: 12px;
          color: var(--ink);
          background: transparent;
          vertical-align: middle;
        }
        .champ-table .champ-p1 td {
          background: color-mix(in srgb, var(--emerald) 16%, var(--surface));
        }
        .champ-table .champ-p1 td:first-child {
          border-radius: 12px 0 0 12px;
        }
        .champ-table .champ-p1 td:last-child {
          border-radius: 0 12px 12px 0;
        }
        .champ-pos {
          color: var(--muted);
          font-weight: 600;
          width: 48px;
        }
        .champ-p1 .champ-pos {
          color: var(--emerald);
        }
        .champ-company {
          display: inline-flex;
          align-items: center;
          gap: 12px;
          font-weight: 600;
        }
        .champ-board .ui-mark {
          display: inline-flex;
          align-items: center;
          justify-content: center;
          width: 28px;
          height: 28px;
          border-radius: 8px;
          color: var(--surface);
          font-size: 13px;
          font-weight: 700;
          flex-shrink: 0;
        }
        .champ-points {
          font-weight: 700;
        }
        .champ-p1 .champ-points {
          color: var(--ink);
          font-size: 1.05em;
        }
        .champ-form h2 {
          margin: 0 0 8px;
          font-size: 15px;
          letter-spacing: 0.02em;
        }
        .champ-form-legend {
          margin: 0 0 16px;
          color: var(--muted);
          font-size: 12px;
          line-height: 1.45;
        }
        .champ-form-list {
          list-style: none;
          margin: 0;
          padding: 0;
          display: flex;
          flex-direction: column;
          gap: 12px;
        }
        .champ-form-list li {
          display: flex;
          align-items: center;
          justify-content: space-between;
          gap: 8px;
        }
        .champ-form-who {
          display: inline-flex;
          align-items: center;
          gap: 8px;
          min-width: 0;
          font-size: 13px;
          font-weight: 600;
        }
        .champ-form-pos {
          color: var(--muted);
          font-weight: 500;
          font-size: 12px;
          width: 24px;
        }
        .champ-form-chips {
          display: inline-flex;
          gap: 4px;
          flex-shrink: 0;
        }
        .champ-chip {
          display: inline-flex;
          align-items: center;
          justify-content: center;
          width: 22px;
          height: 22px;
          border-radius: 6px;
          font-size: 11px;
          font-weight: 700;
        }
        .champ-chip-G {
          background: var(--emerald);
          color: var(--surface);
        }
        .champ-chip-S {
          background: var(--pink);
          color: var(--surface);
        }
        .champ-chip-M {
          background: color-mix(in srgb, var(--muted) 18%, var(--surface));
          color: var(--muted);
        }
        .champ-chip-P {
          background: var(--line);
          color: var(--muted);
        }
        .champ-form-note,
        .champ-updated {
          margin: 16px 0 0;
          color: var(--muted);
          font-size: 12px;
        }
        @media (max-width: 900px) {
          .champ-layout {
            grid-template-columns: 1fr;
          }
          .champ-head {
            flex-direction: column;
            align-items: flex-start;
          }
        }
      `}</style>
    </section>
  );
}
