"use client";

import { useState } from "react";
import styles from "./EnterRace.module.css";

const STEP = 10;
const MIN = 10;
const DEFAULT_AMOUNT = 500;

export default function EnterRace() {
  const [amount, setAmount] = useState(DEFAULT_AMOUNT);
  const [connected, setConnected] = useState(false);

  return (
    <div className={styles.root}>
      <section className={`ui-card ${styles.enter}`} aria-labelledby="enter-race-title">
        <header className={styles.head}>
          <p className={styles.brand}>auctioning.lol</p>
          <p className={styles.kicker} id="enter-race-title">
            <SproutIcon />
            Enter Race
          </p>
        </header>

        <div className={styles.grid}>
          <div className={styles.col}>
            <h2 className={styles.step}>1. Connect wallet</h2>
            <div className={styles.row}>
              <span className={styles.avatar} aria-hidden="true">
                <GhostIcon />
              </span>
              <span className={styles.copy}>
                <strong>Phantom</strong>
                <span>Solana</span>
              </span>
              <button
                type="button"
                className={styles.connect}
                onClick={() => setConnected(true)}
                aria-pressed={connected}
              >
                <span className={styles.dot} data-on={connected ? "1" : "0"} />
                {connected ? "Connected" : "Connect"}
              </button>
            </div>

            <h2 className={styles.step}>2. RP amount</h2>
            <div className={styles.stepper} role="group" aria-label="RP amount">
              <button
                type="button"
                className={styles.nudge}
                onClick={() => setAmount((n) => Math.max(MIN, n - STEP))}
                disabled={amount <= MIN}
                aria-label={`Decrease by ${STEP} RP`}
              >
                −
              </button>
              <p className={styles.amount} aria-live="polite">
                {amount}
              </p>
              <button
                type="button"
                className={styles.nudge}
                onClick={() => setAmount((n) => n + STEP)}
                aria-label={`Increase by ${STEP} RP`}
              >
                +
              </button>
            </div>
            <p className={styles.balance}>RP Balance: 120</p>
          </div>

          <div className={styles.col}>
            <h2 className={styles.step}>3. Payment</h2>
            <div className={styles.row}>
              <span className={`ui-mark ${styles.whop}`} aria-hidden="true">
                W
              </span>
              <span className={styles.copy}>
                <strong>Whop Card</strong>
                <span>Pay securely</span>
              </span>
              <span className={styles.secure}>
                <LockIcon />
                Secure
              </span>
            </div>

            <h2 className={`${styles.step} ${styles.predLabel}`}>
              Live rank prediction
              <InfoIcon />
            </h2>
            <p className={styles.predCopy}>A bid of 410 RP puts you</p>
            <p className={styles.place}>P3</p>
            <p className={styles.live}>
              <LiveArrow />
              Top bids update live
            </p>
          </div>
        </div>

        <button type="button" className={`ui-btn-gradient ${styles.submit}`}>
          Add RP
          <LiveArrow />
        </button>
        <p className={styles.caption}>
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
