import styles from "./RulesGrid.module.css";

export default function RulesGrid() {
  return (
    <section className={styles.section} aria-labelledby="rules-heading">
      <h1 id="rules-heading" className={styles.title}>
        HOW <span>THE</span> RACE WORKS
      </h1>
      <ul className={styles.grid}>
        {RULES.map((rule) => (
          <li key={rule.title} className={`ui-card ${styles.card}`}>
            <header className={styles.cardHead}>
              <h2>{rule.title}</h2>
              <span className={styles.icon} data-tone={rule.tone} aria-hidden="true">
                {rule.icon}
              </span>
            </header>
            <p>{rule.body}</p>
          </li>
        ))}
      </ul>
      <p className={styles.foot}>Built for fun. Driven by community.</p>
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
