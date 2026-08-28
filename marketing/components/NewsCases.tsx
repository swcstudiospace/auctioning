import styles from "./NewsCases.module.css";

type MarkTone = "emerald" | "pink";

type CaseStudy = {
  name: string;
  blurb: string;
  rp: number;
  paid: number;
  community: number;
  letter: string;
  tone: MarkTone;
};

const CASES: CaseStudy[] = [
  {
    name: "Aether Systems",
    blurb: "Lightweight cloud tools for small teams",
    rp: 1240,
    paid: 60,
    community: 40,
    letter: "A",
    tone: "emerald",
  },
  {
    name: "Branchwell",
    blurb: "Simple compliance tracking",
    rp: 980,
    paid: 55,
    community: 45,
    letter: "B",
    tone: "pink",
  },
  {
    name: "Craftory",
    blurb: "No-code templates for freelancers",
    rp: 1710,
    paid: 50,
    community: 50,
    letter: "C",
    tone: "emerald",
  },
  {
    name: "Dashly",
    blurb: "Team status without the meeting",
    rp: 860,
    paid: 40,
    community: 60,
    letter: "D",
    tone: "pink",
  },
  {
    name: "Echoform",
    blurb: "Form builder with built-in analytics",
    rp: 1150,
    paid: 70,
    community: 30,
    letter: "E",
    tone: "emerald",
  },
  {
    name: "Fieldly",
    blurb: "Field service scheduling",
    rp: 760,
    paid: 35,
    community: 65,
    letter: "F",
    tone: "pink",
  },
];

export default function NewsCases() {
  return (
    <section className={styles.section}>
      <header className={styles.header}>
        <h1>How they did it</h1>
        <p>
          Six recent boards. RP burst, then the paid / community mix that
          carried them. Paid RP is emerald. Community RP is pink — and it is
          not money.
        </p>
      </header>

      <div className={styles.grid}>
        {CASES.map((entry) => (
          <article key={entry.name} className={`ui-card ${styles.card}`}>
            <div className={styles.top}>
              <span className={`ui-mark ${styles.mark}`} data-tone={entry.tone}>
                {entry.letter}
              </span>
              <div>
                <h2>{entry.name}</h2>
                <p className={styles.blurb}>{entry.blurb}</p>
              </div>
            </div>

            <p className={styles.rp}>
              <span className={styles.rpValue}>{entry.rp.toLocaleString("en-US")}</span>
              <span className={styles.rpUnit}>RP</span>
            </p>

            <div
              className={styles.mix}
              role="img"
              aria-label={`${entry.paid}% paid RP, ${entry.community}% community RP`}
            >
              <span
                className={styles.mixPaid}
                style={{ width: `${entry.paid}%` }}
              />
              <span
                className={styles.mixCommunity}
                style={{ width: `${entry.community}%` }}
              />
            </div>
            <div className={styles.mixLabels}>
              <span>{entry.paid}% paid</span>
              <span>{entry.community}% community</span>
            </div>
          </article>
        ))}
      </div>
    </section>
  );
}
