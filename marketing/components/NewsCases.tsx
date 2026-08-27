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
    <section className="news-cases">
      <header className="news-head">
        <h1>How they did it</h1>
        <p>
          Six recent boards. RP burst, then the paid / community mix that
          carried them. Paid RP is pink. Community RP is emerald — and it is
          not money.
        </p>
      </header>

      <div className="news-grid">
        {CASES.map((entry) => (
          <article key={entry.name} className="ui-card news-case">
            <div className="news-case-top">
              <span
                className="ui-mark"
                data-tone={entry.tone}
                style={{
                  background:
                    entry.tone === "pink" ? "var(--pink)" : "var(--emerald)",
                }}
              >
                {entry.letter}
              </span>
              <div>
                <h2>{entry.name}</h2>
                <p className="news-blurb">{entry.blurb}</p>
              </div>
            </div>

            <p className="news-rp">
              <span className="news-rp-value">{entry.rp.toLocaleString("en-US")}</span>
              <span className="news-rp-unit">RP</span>
            </p>

            <div
              className="news-mix"
              role="img"
              aria-label={`${entry.paid}% paid RP, ${entry.community}% community RP`}
            >
              <span
                className="news-mix-paid"
                style={{ width: `${entry.paid}%` }}
              />
              <span
                className="news-mix-community"
                style={{ width: `${entry.community}%` }}
              />
            </div>
            <div className="news-mix-labels">
              <span>{entry.paid}% paid</span>
              <span>{entry.community}% community</span>
            </div>
          </article>
        ))}
      </div>

      <style>{`
        .news-cases {
          max-width: 1080px;
          margin: 0 auto;
          padding: 32px 0 48px;
          color: var(--ink);
        }
        .news-head {
          max-width: 52ch;
          margin-bottom: 32px;
        }
        .news-head h1 {
          margin: 0 0 12px;
          font-size: clamp(1.5rem, 3vw, 2rem);
          line-height: 1.2;
          letter-spacing: -0.03em;
          font-weight: 700;
        }
        .news-head p {
          margin: 0;
          color: var(--muted);
          font-size: 15px;
          line-height: 1.55;
        }
        .news-grid {
          display: grid;
          grid-template-columns: repeat(3, minmax(0, 1fr));
          gap: 16px;
        }
        .news-case {
          display: flex;
          flex-direction: column;
          gap: 16px;
          min-width: 0;
        }
        .news-case-top {
          display: flex;
          align-items: flex-start;
          gap: 12px;
        }
        .news-cases .ui-mark {
          display: inline-flex;
          align-items: center;
          justify-content: center;
          width: 32px;
          height: 32px;
          border-radius: 8px;
          color: var(--surface);
          font-size: 14px;
          font-weight: 700;
          flex-shrink: 0;
        }
        .news-case h2 {
          margin: 0 0 4px;
          font-size: 16px;
          letter-spacing: -0.02em;
          font-weight: 700;
        }
        .news-blurb {
          margin: 0;
          color: var(--muted);
          font-size: 13px;
          line-height: 1.45;
        }
        .news-rp {
          margin: auto 0 0;
          display: flex;
          align-items: baseline;
          gap: 8px;
        }
        .news-rp-value {
          font-size: 28px;
          font-weight: 700;
          letter-spacing: -0.03em;
          font-variant-numeric: tabular-nums;
          line-height: 1;
        }
        .news-rp-unit {
          color: var(--muted);
          font-size: 13px;
          font-weight: 600;
        }
        .news-mix {
          display: flex;
          height: 8px;
          border-radius: 999px;
          overflow: hidden;
          background: var(--line);
        }
        .news-mix-paid {
          display: block;
          height: 100%;
          background: var(--pink);
        }
        .news-mix-community {
          display: block;
          height: 100%;
          background: var(--emerald);
        }
        .news-mix-labels {
          display: flex;
          justify-content: space-between;
          gap: 8px;
          color: var(--muted);
          font-size: 12px;
        }
        @media (max-width: 900px) {
          .news-grid {
            grid-template-columns: repeat(2, minmax(0, 1fr));
          }
        }
        @media (max-width: 600px) {
          .news-grid {
            grid-template-columns: 1fr;
          }
        }
      `}</style>
    </section>
  );
}
