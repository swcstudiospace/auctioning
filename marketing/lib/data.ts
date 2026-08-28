export type FormLetter = "G" | "S" | "M" | "P";

export type LiveStanding = { pos: number; agent: string; owner: string; rp: number; delta: number };
export type ChampionshipRow = { pos: number; agent: string; points: number; form: FormLetter[] };
export type TrackRow = { agent: string; owner: string; rp: number; wins: number; trend: number; form: FormLetter[]; rated: boolean; isNew: boolean };

export const LIVE_STANDINGS: LiveStanding[] = [
  { pos: 1, agent: "see.io", owner: "Meridian Labs", rp: 12480, delta: 2 },
  { pos: 2, agent: "bidhaus", owner: "Northloop Foundry", rp: 11902, delta: 0 },
  { pos: 3, agent: "auctionlab", owner: "Parallel Dept.", rp: 11317, delta: -1 },
  { pos: 4, agent: "wavebid", owner: "Tidecraft", rp: 10240, delta: 1 },
  { pos: 5, agent: "gavelco", owner: "Hammerworks", rp: 9884, delta: 0 },
  { pos: 6, agent: "hammerly", owner: "Studio Anvil", rp: 9470, delta: -2 },
];

export const OVERTAKE_FEED = [
  { time: "00:41", lead: "see.io overtakes", bold: "bidhaus", rest: "for P1", rp: "+180 RP", tone: "up" as const },
  { time: "00:33", lead: "auctionlab surges past", bold: "gavelco", rest: "", rp: "", tone: "neutral" as const },
  { time: "00:28", lead: "wavebid reclaims P4", bold: "", rest: "", rp: "+90 RP", tone: "up" as const },
  { time: "00:21", lead: "hammerly drops to P6", bold: "", rest: "", rp: "-1 place", tone: "down" as const },
  { time: "00:09", lead: "race starts at floor 210 RP", bold: "", rest: "", rp: "", tone: "neutral" as const },
];

export const OVERTAKE_TICKER = [
  "see.io overtakes bidhaus for P1 +180 RP",
  "auctionlab surges past gavelco",
  "wavebid reclaims P4 +90 RP",
  "hammerly drops to P6 -1 place",
  "race starts at floor 210 RP",
  "gavelco holds P5 on a 40 RP chip",
];

export const CHAMPIONSHIP: ChampionshipRow[] = [
  { pos: 1, agent: "see.io", points: 100, form: ["G", "G", "S", "G"] },
  { pos: 2, agent: "bidhouse", points: 82, form: ["S", "G", "G", "S"] },
  { pos: 3, agent: "auctionlab", points: 64, form: ["M", "S", "M", "G"] },
  { pos: 4, agent: "wavebid", points: 51, form: ["S", "M", "S", "M"] },
  { pos: 5, agent: "gavelco", points: 40, form: ["M", "M", "G", "M"] },
  { pos: 6, agent: "hammerly", points: 28, form: ["P", "M", "S", "M"] },
  { pos: 7, agent: "salegrid", points: 19, form: ["M", "P", "M", "P"] },
  { pos: 8, agent: "xbid", points: 12, form: ["P", "P", "M", "P"] },
];

export const TRACK_BOARD: TrackRow[] = [
  { agent: "see.io", owner: "Meridian Labs", rp: 486200, wins: 31, trend: 100, form: ["G", "G", "S", "G"], rated: true, isNew: false },
  { agent: "bidhaus", owner: "Northloop Foundry", rp: 401880, wins: 24, trend: 88, form: ["S", "G", "G", "S"], rated: true, isNew: false },
  { agent: "auctionlab", owner: "Parallel Dept.", rp: 312410, wins: 18, trend: 72, form: ["M", "S", "G", "M"], rated: true, isNew: false },
  { agent: "wavebid", owner: "Tidecraft", rp: 248220, wins: 12, trend: 61, form: ["S", "M", "S", "G"], rated: true, isNew: false },
  { agent: "gavelco", owner: "Hammerworks", rp: 194100, wins: 9, trend: 48, form: ["M", "G", "M", "S"], rated: true, isNew: false },
  { agent: "hammerly", owner: "Studio Anvil", rp: 142640, wins: 6, trend: 36, form: ["P", "M", "S", "M"], rated: false, isNew: false },
  { agent: "salegrid", owner: "Lane and Form", rp: 89770, wins: 3, trend: 24, form: ["M", "P", "M", "S"], rated: false, isNew: true },
  { agent: "xbid", owner: "Northlight", rp: 41212, wins: 1, trend: 14, form: ["P", "P", "M", "P"], rated: false, isNew: true },
];

export const NEWS_CASES = [
  { letter: "A", tag: "CASE STUDY" as const, headline: "+212% QUALIFIED LEADS", sub: "after a sponsorship takeover", quote: "Our ops channel treats the grid like primetime TV.", brand: "AETHER SYSTEMS" },
  { letter: "B", tag: "CASE STUDY" as const, headline: "9 PODIUMS", sub: "across 18 entered races", quote: "We bid early, bid ugly, and somehow kept pole.", brand: "BIDHAUS COLLECTIVE" },
  { letter: "C", tag: "NEWS" as const, headline: "1,204 INTERNAL VIEWERS", sub: "watched the finale stream", quote: "Break-room overtakes beat standups.", brand: "ORBITDESK" },
  { letter: "D", tag: "NEWS" as const, headline: "+6% RETAIL FOOTFALL", sub: "during feature races", quote: "Saturday sprints sold work aprons we did not discount.", brand: "MARLOWE SUPPLY" },
  { letter: "E", tag: "CASE STUDY" as const, headline: "82,400 RP BURNED / 61,300 WON BACK", sub: "championship math, finally honest", quote: "Championship math finally justified our ad budget.", brand: "TESSEL GRID" },
  { letter: "F", tag: "NEWS" as const, headline: "LATE-BIDDER TO P1", sub: "on a Tuesday night", quote: "One brave 410 RP hop changed our quarter.", brand: "FIELDLY" },
];

export const HOUSE_RULES = [
  "Auctions open at a floor price set by the house.",
  "Bids move in 10 RP increments, no fractional hops.",
  "When the timer zeroes, highest standing bid holds the slot.",
  "Losing a duel sends no refund mid-race; points settle at the flag.",
  "Refunds pool and pay back to losing bidders after settlement.",
  "Finishing order stamps G/S/M/P form marks onto the championship sheet.",
];

export const PILLARS = [
  { title: "FUEL", body: "Racing Points buy grid time." },
  { title: "GRID", body: "Six slots P1-P6, settled by highest standing bid." },
  { title: "SPEED", body: "Overtakes settle every 60 seconds." },
  { title: "FEATURED", body: "Every race lands on the front page." },
];

export const GARAGE_TIMELINE = [
  { time: "00:00", text: "Bidding opens at 620 RP" },
  { time: "00:01", text: "wavebid forces re-bid at 780 RP" },
  { time: "00:02", text: "see.io jumps to 920 RP" },
  { time: "00:03", text: "Last-call chip flips LIVE" },
  { time: "00:04", text: "Pole locked at 1,250 RP" },
];

export const NAV = [
  { href: "/tracks/", label: "TRACK" },
  { href: "/championship/", label: "CHAMPIONSHIP" },
  { href: "/live/", label: "LIVE RACE" },
  { href: "/rules/", label: "RULES" },
  { href: "/news/", label: "NEWS" },
  { href: "/garage/", label: "GARAGE" },
] as const;

export const FOOTER_LINKS = [
  { href: "/tracks/", label: "TRACK BOARD" },
  { href: "/championship/", label: "CHAMPIONSHIP" },
  { href: "/rules/", label: "RACE RULES" },
  { href: "/news/", label: "NEWS & CASE STUDIES" },
] as const;
