"use client";

import { useState } from "react";

const STEP = 10;
const MIN = 10;
const DEFAULT_AMOUNT = 500;

export default function EnterRace() {
  const [amount, setAmount] = useState(DEFAULT_AMOUNT);
  const [connected, setConnected] = useState(false);

  return (
    <div className="ui-enter-root">
      <style>{css}</style>
      <section className="ui-card ui-enter" aria-labelledby="enter-race-title">
        <header className="ui-enter-head">
          <p className="ui-enter-brand">auctioning.lol</p>
          <p className="ui-enter-kicker" id="enter-race-title">
            <SproutIcon />
            Enter Race
          </p>
        </header>

        <div className="ui-enter-grid">
          <div className="ui-enter-col">
            <h2 className="ui-enter-step">1. Connect wallet</h2>
            <div className="ui-enter-row">
              <span className="ui-enter-avatar" aria-hidden="true">
                <GhostIcon />
              </span>
              <span className="ui-enter-copy">
                <strong>Phantom</strong>
                <span>Solana</span>
              </span>
              <button
                type="button"
                className="ui-enter-connect"
                onClick={() => setConnected(true)}
                aria-pressed={connected}
              >
                <span className="ui-enter-dot" data-on={connected ? "1" : "0"} />
                {connected ? "Connected" : "Connect"}
              </button>
            </div>

            <h2 className="ui-enter-step">2. RP amount</h2>
            <div className="ui-enter-stepper" role="group" aria-label="RP amount">
              <button
                type="button"
                className="ui-enter-nudge"
                onClick={() => setAmount((n) => Math.max(MIN, n - STEP))}
                disabled={amount <= MIN}
                aria-label={`Decrease by ${STEP} RP`}
              >
                −
              </button>
              <p className="ui-enter-amount" aria-live="polite">
                {amount}
              </p>
              <button
                type="button"
                className="ui-enter-nudge"
                onClick={() => setAmount((n) => n + STEP)}
                aria-label={`Increase by ${STEP} RP`}
              >
                +
              </button>
            </div>
            <p className="ui-enter-balance">RP Balance: 120</p>
          </div>

          <div className="ui-enter-col">
            <h2 className="ui-enter-step">3. Payment</h2>
            <div className="ui-enter-row">
              <span className="ui-mark ui-enter-whop" aria-hidden="true">
                W
              </span>
              <span className="ui-enter-copy">
                <strong>Whop Card</strong>
                <span>Pay securely</span>
              </span>
              <span className="ui-enter-secure">
                <LockIcon />
                Secure
              </span>
            </div>

            <h2 className="ui-enter-step ui-enter-pred-label">
              Live rank prediction
              <InfoIcon />
            </h2>
            <p className="ui-enter-pred-copy">A bid of 410 RP puts you</p>
            <p className="ui-enter-place">P3</p>
            <p className="ui-enter-live">
              <LiveArrow />
              Top bids update live
            </p>
          </div>
        </div>

        <button type="button" className="ui-btn-gradient ui-enter-submit">
          Add RP
          <LiveArrow />
        </button>
        <p className="ui-enter-caption">
          RP is non-refundable. For race use only.
        </p>
      </section>
    </div>
  );
}

function SproutIcon() {
  return (
    <svg viewBox="0 0 24 24" width="16" height="16" aria-hidden="true">
      <path
        d="M12 21V11"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.8"
        strokeLinecap="round"
      />
      <path
        d="M12 12c0-4 3-7 8-8-1 5-4 8-8 8Z"
        fill="currentColor"
        opacity="0.9"
      />
      <path
        d="M12 14c0-3.2-2.4-6-6.5-7 1.2 4 3.4 6.4 6.5 7Z"
        fill="currentColor"
        opacity="0.55"
      />
    </svg>
  );
}

function GhostIcon() {
  return (
    <svg viewBox="0 0 32 32" width="26" height="26" aria-hidden="true">
      <path
        d="M16 4c-5.4 0-9 4.2-9 10.2V24l2.4-1.6 2.6 1.6 2.5-1.6 2.5 1.6 2.6-1.6 2.4 1.6V14.2C25 8.2 21.4 4 16 4Z"
        fill="var(--muted)"
        opacity="0.35"
      />
      <circle cx="12.2" cy="14" r="1.5" fill="var(--ink)" />
      <circle cx="19.8" cy="14" r="1.5" fill="var(--ink)" />
    </svg>
  );
}

function LockIcon() {
  return (
    <svg viewBox="0 0 16 16" width="12" height="12" aria-hidden="true">
      <rect
        x="3"
        y="7"
        width="10"
        height="7"
        rx="1.4"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.4"
      />
      <path
        d="M5.2 7V5.4a2.8 2.8 0 0 1 5.6 0V7"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.4"
        strokeLinecap="round"
      />
    </svg>
  );
}

function InfoIcon() {
  return (
    <svg viewBox="0 0 16 16" width="13" height="13" aria-hidden="true">
      <circle
        cx="8"
        cy="8"
        r="6.2"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.3"
      />
      <path
        d="M8 7.2V12"
        stroke="currentColor"
        strokeWidth="1.4"
        strokeLinecap="round"
      />
      <circle cx="8" cy="5" r="0.9" fill="currentColor" />
    </svg>
  );
}

function LiveArrow() {
  return (
    <svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true">
      <path
        d="M4 12 12 4"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.6"
        strokeLinecap="round"
      />
      <path
        d="M6 4h6v6"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.6"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

const css = `
.ui-enter-root {
  width: min(100%, 44rem);
  margin: 0 auto;
}
.ui-enter {
  padding: 1.75rem 1.75rem 1.35rem;
  color: var(--ink);
}
.ui-enter-head {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 1rem;
  margin-bottom: 1.75rem;
}
.ui-enter-brand {
  margin: 0;
  font-size: 1.05rem;
  font-weight: 600;
  letter-spacing: -0.03em;
}
.ui-enter-kicker {
  margin: 0;
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
  color: var(--emerald);
  font-size: 0.82rem;
  font-weight: 600;
}
.ui-enter-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 1.75rem 2rem;
}
.ui-enter-col {
  min-width: 0;
}
.ui-enter-step {
  margin: 0 0 0.65rem;
  color: var(--muted);
  font-size: 0.68rem;
  font-weight: 600;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}
.ui-enter-col > .ui-enter-step:not(:first-child) {
  margin-top: 1.35rem;
}
.ui-enter-row {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  min-height: 3.35rem;
  padding: 0.55rem 0.75rem;
  border: 1px solid var(--line);
  border-radius: 0.9rem;
  background: var(--surface);
}
.ui-enter-avatar {
  display: grid;
  place-items: center;
  width: 2.35rem;
  height: 2.35rem;
  border-radius: 999px;
  background: var(--bg);
  flex-shrink: 0;
}
.ui-enter-whop {
  display: grid;
  place-items: center;
  width: 2.15rem;
  height: 2.15rem;
  border-radius: 0.55rem;
  background: var(--ink);
  color: var(--surface);
  font-size: 0.95rem;
  font-weight: 700;
  flex-shrink: 0;
}
.ui-enter-copy {
  display: flex;
  flex-direction: column;
  gap: 0.05rem;
  min-width: 0;
  flex: 1;
}
.ui-enter-copy strong {
  font-size: 0.92rem;
  font-weight: 650;
  letter-spacing: -0.02em;
}
.ui-enter-copy span {
  color: var(--muted);
  font-size: 0.75rem;
}
.ui-enter-connect {
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
  margin: 0;
  padding: 0.2rem 0.15rem;
  border: 0;
  background: transparent;
  color: var(--emerald);
  font: inherit;
  font-size: 0.82rem;
  font-weight: 650;
  cursor: pointer;
  border-radius: 0.3rem;
}
.ui-enter-connect:hover {
  color: var(--ink);
}
.ui-enter-connect:focus-visible,
.ui-enter-nudge:focus-visible,
.ui-enter-submit:focus-visible {
  outline: 2px solid var(--emerald);
  outline-offset: 2px;
}
.ui-enter-dot {
  width: 0.45rem;
  height: 0.45rem;
  border-radius: 999px;
  background: var(--line);
}
.ui-enter-dot[data-on="1"] {
  background: var(--emerald);
}
.ui-enter-secure {
  display: inline-flex;
  align-items: center;
  gap: 0.28rem;
  color: var(--muted);
  font-size: 0.75rem;
  font-weight: 600;
}
.ui-enter-stepper {
  display: grid;
  grid-template-columns: 2.75rem 1fr 2.75rem;
  border: 1px solid var(--line);
  border-radius: 0.7rem;
  overflow: hidden;
  background: var(--surface);
}
.ui-enter-nudge {
  margin: 0;
  border: 0;
  background: var(--surface);
  color: var(--ink);
  font: inherit;
  font-size: 1.15rem;
  line-height: 1;
  cursor: pointer;
  min-height: 2.85rem;
}
.ui-enter-nudge:hover:not(:disabled) {
  background: var(--bg);
}
.ui-enter-nudge:disabled {
  color: var(--line);
  cursor: not-allowed;
}
.ui-enter-nudge:first-child {
  border-right: 1px solid var(--line);
}
.ui-enter-nudge:last-child {
  border-left: 1px solid var(--line);
}
.ui-enter-amount {
  margin: 0;
  display: grid;
  place-items: center;
  font-size: 1.05rem;
  font-weight: 650;
  font-variant-numeric: tabular-nums;
  letter-spacing: -0.03em;
}
.ui-enter-balance {
  margin: 0.55rem 0 0;
  color: var(--muted);
  font-size: 0.78rem;
}
.ui-enter-pred-label {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
}
.ui-enter-pred-label svg {
  color: var(--muted);
}
.ui-enter-pred-copy {
  margin: 0.15rem 0 0;
  color: var(--muted);
  font-size: 0.9rem;
}
.ui-enter-place {
  margin: 0.15rem 0 0.55rem;
  color: var(--emerald);
  font-size: 2.35rem;
  font-weight: 700;
  letter-spacing: -0.06em;
  line-height: 1;
}
.ui-enter-live {
  margin: 0;
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  color: var(--muted);
  font-size: 0.78rem;
}
.ui-enter-submit {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 0.5rem;
  width: 100%;
  margin-top: 1.6rem;
  min-height: 3.1rem;
  border: 0;
  cursor: pointer;
  font: inherit;
  font-size: 0.95rem;
  font-weight: 700;
  letter-spacing: 0.12em;
  text-transform: uppercase;
}
.ui-enter-caption {
  margin: 0.85rem 0 0;
  text-align: center;
  color: var(--muted);
  font-size: 0.78rem;
}
@media (max-width: 700px) {
  .ui-enter {
    padding: 1.25rem 1.15rem 1.1rem;
  }
  .ui-enter-grid {
    grid-template-columns: 1fr;
    gap: 0.35rem;
  }
  .ui-enter-col > .ui-enter-step:not(:first-child) {
    margin-top: 1.15rem;
  }
  .ui-enter-place {
    font-size: 2rem;
  }
}
`;
