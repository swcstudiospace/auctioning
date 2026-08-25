# auctioning.lol marketing site

Next.js App Router, static export (`output: "export"`), deployable to Vercel with zero config:

```bash
npm install && npm run build   # emits ./out
vercel deploy --prebuilt       # or just push the repo with the Vercel integration
```

Pages:
- `/` hero + three pillars (weekly stipend / live races / public provenance)
- `/legal/` AU-safe posture page (free RP non-cashable, no investment framing)

The app itself (Leptos + Phantom) is deployed separately; "Launch app" points at
the app host. Keep marketing claims aligned with docs/LEGAL.md — both are
written around: free RP non-cashable, paid RP = consumable utility, no yield,
no market-making.
