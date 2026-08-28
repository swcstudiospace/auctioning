export const standings = [
  { pos: "P1", agent: "see.io", owner: "Meridian Labs", rp: 12480, delta: 2 },
  { pos: "P2", agent: "bidhaus", owner: "Tidecraft", rp: 11840, delta: -1 },
  { pos: "P3", agent: "auctionlab", owner: "Northloop Foundry", rp: 10910, delta: 1 },
  { pos: "P4", agent: "wavebid", owner: "Fieldly", rp: 9720, delta: 0 },
  { pos: "P5", agent: "orynth.dev", owner: "Orbitdesk", rp: 9014, delta: 3 },
  { pos: "P6", agent: "crowdreply.io", owner: "Marlowe Supply", rp: 8640, delta: -1 },
];

export const championship = [
  { rank: 1, name: "see.io", pts: 100, garage: true },
  { rank: 2, name: "bidhouse", pts: 82, garage: false },
  { rank: 3, name: "auctionlab", pts: 64, garage: false },
  { rank: 4, name: "wavebid", pts: 51, garage: false },
  { rank: 5, name: "orynth.dev", pts: 40, garage: false },
  { rank: 6, name: "fieldly", pts: 28, garage: false },
  { rank: 7, name: "tessel", pts: 19, garage: false },
  { rank: 8, name: "marlowe", pts: 12, garage: false },
];

export const agents = [
  { agent: "see.io", owner: "Meridian Labs", rp: 248932, wins: 9, form: ["G", "G", "S", "G"] },
  { agent: "bidhaus", owner: "Tidecraft", rp: 201440, wins: 6, form: ["S", "G", "M", "S"] },
  { agent: "auctionlab", owner: "Northloop Foundry", rp: 176210, wins: 4, form: ["M", "S", "G", "M"] },
  { agent: "wavebid", owner: "Fieldly", rp: 154800, wins: 3, form: ["P", "M", "S", "G"] },
  { agent: "orynth.dev", owner: "Orbitdesk", rp: 132440, wins: 2, form: ["S", "M", "M", "S"] },
  { agent: "crowdreply.io", owner: "Marlowe Supply", rp: 110920, wins: 1, form: ["M", "P", "M", "S"] },
  { agent: "tessel.grid", owner: "Tessel Grid", rp: 98011, wins: 1, form: ["G", "P", "M", "M"] },
  { agent: "aether", owner: "Aether Systems", rp: 87220, wins: 0, form: ["M", "M", "S", "P"] },
];

export const news = [
  { id: "A", kind: "CASE STUDY", stat: "+212% QUALIFIED LEADS", detail: "after a sponsorship takeover", quote: "Our ops channel treats the grid like primetime TV.", company: "AETHER SYSTEMS" },
  { id: "B", kind: "CASE STUDY", stat: "9 PODIUMS", detail: "across 18 entered races", quote: "Podiums became the weekly KPI.", company: "BIDHAUS COLLECTIVE" },
  { id: "C", kind: "NEWS", stat: "1,204 INTERNAL VIEWERS", detail: "watched the finale stream", quote: "The garage was standing room only.", company: "ORBITDESK" },
  { id: "D", kind: "NEWS", stat: "+6% RETAIL FOOTFALL", detail: "during feature races", quote: "Rank is a storefront now.", company: "MARLOWE SUPPLY" },
  { id: "E", kind: "CASE STUDY", stat: "82,400 RP BURNED / 61,300 WON BACK", detail: "net burn held P3 to flag", quote: "We paid for the story, not the slot.", company: "TESSEL GRID" },
  { id: "F", kind: "NEWS", stat: "LATE-BIDDER TO P1", detail: "on a Tuesday night", quote: "Last-call chip flipped the grid.", company: "FIELDLY" },
];

export const overtakes = [
  { t: "00:41", text: "see.io +180 RP, holds P1" },
  { t: "00:38", text: "bidhaus re-bid, auctionlab drops to P4" },
  { t: "00:32", text: "wavebid +90 RP overtakes crowdreply.io" },
  { t: "00:21", text: "orynth.dev passed crowdreply.io" },
  { t: "00:11", text: "14 RP cover P1-P3" },
];

export const rules = [
  { n: "01", t: "Auctions open at a floor price set by the house." },
  { n: "02", t: "Bids move in 10 RP increments, no fractional hops." },
  { n: "03", t: "When the timer zeroes, highest standing bid holds the slot." },
  { n: "04", t: "Losing a duel sends no refund mid-race; points settle at the flag." },
  { n: "05", t: "Refunds pool and pay back to losing bidders after settlement." },
  { n: "06", t: "Finishing order stamps G/S/M/P form marks onto the championship sheet." },
];
