export type BlogPost = {
  slug: string;
  title: string;
  date: string;
  dateLabel: string;
  author: string;
  kicker: string;
  excerpt: string;
  body: string;
};

export const posts: BlogPost[] = [
  {
    slug: "launching-auctioning-lol",
    title: "Auctioning.lol is live",
    date: "2026-08-29",
    dateLabel: "29 Aug 2026",
    author: "Oveshen Govender",
    kicker: "LAUNCH",
    excerpt:
      "Play-to-rank for companies. RP is fuel. Rank is the scoreboard. News is the product.",
    body: `The board is up.

Auctioning.lol is a play-to-rank racing network for companies. You do not buy a listing. You fuel one. Rank is what the ledger says it is, not a press release.

## Why this exists

Most company leaderboards are ads wearing a table. Auctioning is the opposite. Position is earned in Racing Points. The catalog is public. The garage is telemetry. The championship is points, not a paid badge.

The first grid is the outbid.lol catalog: 2,089 companies, imported as-is, every one sitting at 0 RP. Nobody inherited rank. The first dollar of fuel, and the first weekly claim, write the first real position.

## How it works

**RANK** is the catalog. Search it. Page it. Hover a row for the briefing. Open the garage for the full telemetry.

**RP is fuel.** Paid RP is one dollar, one point, logged on-chain through Phantom. Community RP is a weekly claim. It is not money and it does not cash out.

**Championship** is a points table. Grand Prix and sprint scoring, same shape as a real grid: P1–P10, sprint P1–P3, fastest pace +1. Empty until a window archives. We will not fill it with fake standings.

**LIVE** is one race at a time. The calendar rail is sprints, GPs, and whoever is actually leading championship.

**NEWS** is this blog. House posts go here. Ledger recaps go here when an overtake, photo finish, or archived window writes a row. No invented ROI. No fake case studies.

## What garage is

Garage is not a tab. It is the company page. Click a listing, get the dashboard: racing metrics, BI metrics, the story. Hover is the briefing. Garage is the rest.

## What we will not do

We will not seed RP so the board looks busy. We will not invent clicks, CPC, or form. We will not cash out community RP. We will not pretend championship has a leader until a window archives.

If you want #1, fuel it.

[Claim #1](/rank#claim)`,
  },
];

export function getPost(slug: string) {
  return posts.find((p) => p.slug === slug);
}

export function allPosts() {
  return [...posts].sort((a, b) => (a.date < b.date ? 1 : -1));
}
