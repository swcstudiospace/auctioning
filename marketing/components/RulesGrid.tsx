export default function RulesGrid() {
  return (
    <section className="ui-rules" aria-labelledby="rules-heading">
      <style>{css}</style>
      <h1 id="rules-heading" className="ui-rules-title">
        HOW <span>THE</span> RACE WORKS
      </h1>
      <ul className="ui-rules-grid">
        {RULES.map((rule) => (
          <li key={rule.title} className="ui-card ui-rules-card">
            <header className="ui-rules-card-head">
              <h2>{rule.title}</h2>
              <span className="ui-rules-icon" data-tone={rule.tone} aria-hidden="true">
                {rule.icon}
              </span>
            </header>
            <p>{rule.body}</p>
          </li>
        ))}
      </ul>
      <p className="ui-rules-foot">Built for fun. Driven by community.</p>
    </section>
  );
}

const RULES = [
  {
    title: "Fuel",
    tone: "emerald",
    body: "Buy RP. $1 = 1 RP. Fuel the race.",
    icon: <FuelIcon />,
  },
  {
    title: "Grid",
    tone: "emerald",
    body: "Sixteen slots. One live board.",
    icon: <FlagIcon />,
  },
  {
    title: "Speed",
    tone: "pink",
    body: "~50ms ticks. MagicBlock L2.",
    icon: <BoltIcon />,
  },
  {
    title: "Featured",
    tone: "pink",
    body: "Winner earns the homepage.",
    icon: <StarIcon />,
  },
];

function FuelIcon() {
  return (
    <svg viewBox="0 0 32 32" width="28" height="28">
      <rect
        x="6"
        y="8"
        width="12"
        height="18"
        rx="2"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.8"
      />
      <rect x="8.5" y="11" width="7" height="5" rx="1" fill="currentColor" opacity="0.35" />
      <path
        d="M18 12h3.2c.7 0 1.3.4 1.6 1l2.4 4.2c.2.4.3.8.3 1.2V22a3 3 0 0 1-3 3"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.8"
        strokeLinecap="round"
      />
      <circle cx="22.5" cy="25" r="2.2" fill="none" stroke="currentColor" strokeWidth="1.8" />
      <path d="M10 26v2" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" />
    </svg>
  );
}

function FlagIcon() {
  return (
    <svg viewBox="0 0 32 32" width="28" height="28">
      <path
        d="M8 5v22"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.8"
        strokeLinecap="round"
      />
      <path
        d="M8 6h16l-3.2 5L24 16H8"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.8"
        strokeLinejoin="round"
      />
      <path d="M11 8h3v3h-3zm5 0h3v3h-3zm-2 5h3v3h-3z" fill="currentColor" opacity="0.45" />
    </svg>
  );
}

function BoltIcon() {
  return (
    <svg viewBox="0 0 32 32" width="28" height="28">
      <path
        d="M18 4 8 18h7l-2 10 12-16h-7L18 4Z"
        fill="currentColor"
        opacity="0.92"
      />
    </svg>
  );
}

function StarIcon() {
  return (
    <svg viewBox="0 0 32 32" width="28" height="28">
      <path
        d="M16 5.5 18.9 13h7.6l-6.1 4.6 2.3 7.4L16 20.6 9.3 25l2.3-7.4L5.5 13h7.6L16 5.5Z"
        fill="currentColor"
      />
    </svg>
  );
}

const css = `
.ui-rules {
  width: min(100%, 68rem);
  margin: 0 auto;
  color: var(--ink);
}
.ui-rules-title {
  margin: 0 0 1.75rem;
  font-size: clamp(1.65rem, 4vw, 2.55rem);
  font-weight: 700;
  letter-spacing: -0.045em;
  line-height: 1.05;
  text-transform: uppercase;
}
.ui-rules-title span {
  color: var(--pink);
}
.ui-rules-grid {
  list-style: none;
  margin: 0;
  padding: 0;
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 1rem;
}
.ui-rules-card {
  padding: 1.15rem 1.15rem 1.3rem;
  min-height: 11.5rem;
}
.ui-rules-card-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 0.75rem;
  margin-bottom: 1.5rem;
}
.ui-rules-card h2 {
  margin: 0;
  font-size: 1.35rem;
  font-weight: 650;
  letter-spacing: -0.03em;
}
.ui-rules-icon {
  display: grid;
  place-items: center;
  width: 2.5rem;
  height: 2.5rem;
  flex-shrink: 0;
}
.ui-rules-icon[data-tone="emerald"] {
  color: var(--emerald);
}
.ui-rules-icon[data-tone="pink"] {
  color: var(--pink);
}
.ui-rules-card p {
  margin: 0;
  color: var(--muted);
  font-size: 0.92rem;
  line-height: 1.45;
  max-width: 16ch;
}
.ui-rules-foot {
  margin: 2.5rem 0 0;
  color: var(--muted);
  font-size: 0.92rem;
}
@media (max-width: 900px) {
  .ui-rules-grid {
    grid-template-columns: 1fr 1fr;
  }
  .ui-rules-card p {
    max-width: none;
  }
}
@media (max-width: 560px) {
  .ui-rules-grid {
    grid-template-columns: 1fr;
  }
  .ui-rules-card {
    min-height: 0;
  }
}
`;
