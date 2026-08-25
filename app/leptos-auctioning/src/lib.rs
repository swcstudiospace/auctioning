//! Leptos app for auctioning.lol — wallet + RP flows.
//!
//! CSR-only build (`make_app`): `trunk serve` for dev, `trunk build --release`
//! for a static bundle served by any CDN. The backend API base is injected at
//! build time via `AUCTIONING_API_BASE` (defaults to the Shuttle prod URL).

use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsCast;

/// Backend API base URL. Overridable at build time:
///   AUCTIONING_API_BASE=https://auctioning-backend.shuttle.app trunk build
pub fn api_base() -> String {
    option_env!("AUCTIONING_API_BASE")
        .unwrap_or("https://auctioning-backend.shuttle.app")
        .to_string()
}

// ---------------------------------------------------------------------------
// Phantom wallet bridge (window.solana / window.phantom.solana)
// ---------------------------------------------------------------------------

/// The subset of the Phantom provider we use. Kept as raw JS interop because
/// Phantom's injected object is not serializable into a Rust struct.
#[derive(Clone, Copy)]
pub struct Phantom;

pub const PHANTOM_INSTALL_URL: &str = "https://phantom.app/";

impl Phantom {
    /// Whether a Solana provider is injected in this window.
    pub fn available() -> bool {
        !js_sys::eval(
            "(function(){const p=window.phantom?.solana||window.solana;return !!(p&&p.isPhantom)})()",
        )
        .ok()
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    }

    /// Prompt Phantom to connect; returns the first public key (base58).
    pub async fn connect() -> Result<String, String> {
        let promise: js_sys::Promise = js_sys::eval(
            "(async function(){const p=window.phantom?.solana||window.solana;\\
             if(!p)throw new Error('phantom-not-found');\\
             const r=await p.connect();\\
             return r.publicKey?r.publicKey.toString():'';})()",
        )
        .map_err(|_| "js-eval-failed".to_string())?
        .dyn_into()
        .map_err(|_| "not-a-promise")?;
        let value = wasm_bindgen_futures::JsFuture::from(promise)
            .await
            .map_err(|e| format!("connect rejected: {e:?}"))?;
        value
            .as_string()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| "no public key returned".to_string())
    }

    /// Ask Phantom to sign an arbitrary UTF-8 message (durable nonce for
    /// session binding). Returns the base58 signature.
    pub async fn sign_message(wallet: &str, message: &str) -> Result<String, String> {
        let script = format!(
            "(async function(){{const p=window.phantom?.solana||window.solana;\\
             const enc=new TextEncoder();\\
             const r=await p.signMessage(enc.encode({message:?}),'utf8');\\
             const bs58=(await import('bs58')).default;\\
             return bs58.encode(r.signature);}})()"
        );
        // `wallet` is bound into the session context server-side; the message
        // itself carries the challenge.
        let _ = wallet;
        let promise: js_sys::Promise = js_sys::eval(&script)
            .map_err(|_| "js-eval-failed".to_string())?
            .dyn_into()
            .map_err(|_| "not-a-promise")?;
        let value = wasm_bindgen_futures::JsFuture::from(promise)
            .await
            .map_err(|e| format!("sign rejected: {e:?}"))?;
        value
            .as_string()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| "no signature returned".to_string())
    }

    /// Sign and send a base64-encoded Transaction from backend prepare.
    pub async fn send_transaction(_wallet: &str, tx_base64: &str) -> Result<String, String> {
        let script = format!(
            "(async function(){{ \
                const p = window.phantom?.solana || window.solana; \
                if (!p) throw new Error('no-phantom'); \
                const txBytes = Uint8Array.from(atob({tx_base64:?}), c => c.charCodeAt(0)); \
                let result; \
                if (window.solanaWeb3 && window.solanaWeb3.Transaction) {{ \
                    const tx = window.solanaWeb3.Transaction.from(txBytes); \
                    result = await p.signAndSendTransaction(tx); \
                }} else {{ \
                    result = await p.signAndSendTransaction({{ transaction: txBytes }}); \
                }} \
                return result.signature; \
            }})()"
        );
        let promise: js_sys::Promise = js_sys::eval(&script)
            .map_err(|_| "js-eval-failed".to_string())?
            .dyn_into()
            .map_err(|_| "not-a-promise")?;
        let value = wasm_bindgen_futures::JsFuture::from(promise)
            .await
            .map_err(|e| format!("send rejected: {e:?}"))?;
        value.as_string().filter(|s| !s.is_empty()).ok_or_else(|| "no signature".to_string())
    }
}

// ---------------------------------------------------------------------------
// API client (Shuttle backend)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpView {
    pub wallet: String,
    pub paid_rp: i64,
    pub free_rp: i64,
    pub spent_rp: i64,
    pub free_rp_non_cashable: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct WeeklyClaimRequest<'a> {
    wallet: &'a str,
}

#[derive(Debug, Serialize, Deserialize)]
struct SpendRequest<'a> {
    wallet: &'a str,
    amount: i64,
    reason: &'a str,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectRow {
    pub handle: String,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub blurb: Option<String>,
    pub total_rp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridSlot {
    pub handle: String,
    pub rank: i32,
    pub race_rp: i64,
    pub velocity: i64,
    pub momentum: i64,
    pub gap_to_leader: i64,
    pub gap_to_next: Option<i64>,
}

async fn api_get(path: &str) -> Result<reqwest::Response, String> {
    reqwest::Client::new()
        .get(format!("{}{path}", api_base()))
        .send()
        .await
        .map_err(|e| format!("network error: {e}"))
}

async fn api_post<T: serde::Serialize>(path: &str, body: &T) -> Result<reqwest::Response, String> {
    reqwest::Client::new()
        .post(format!("{}{path}", api_base()))
        .json(body)
        .send()
        .await
        .map_err(|e| format!("network error: {e}"))
}

// ---------------------------------------------------------------------------
// UI components
// ---------------------------------------------------------------------------

/// Top-level app shell with the connect button + main flows.
#[component]
pub fn App() -> impl IntoView {
    let wallet = RwSignal::new(None::<String>);
    let rp = RwSignal::new(None::<RpView>);
    let busy = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);
    let notice = RwSignal::new(None::<String>);

    // Load RP view whenever the wallet changes.
    Effect::new(move |_| {
        let w = wallet.get();
        if let Some(addr) = w {
            spawn_local(async move {
                match api_get(&format!("/v1/rp/{addr}")).await {
                    Ok(resp) if resp.status().is_success() => {
                        if let Ok(v) = resp.json::<RpView>().await {
                            rp.set(Some(v));
                        }
                    }
                    Ok(resp) => error.set(Some(format!("backend {resp:?}"))),
                    Err(e) => error.set(Some(e)),
                }
            });
        } else {
            rp.set(None);
        }
    });

    let on_connect = move |_| {
        spawn_local(async move {
            busy.set(true);
            error.set(None);
            if !Phantom::available() {
                notice.set(Some(format!(
                    "Phantom not detected. Install it at {PHANTOM_INSTALL_URL}"
                )));
                busy.set(false);
                return;
            }
            match Phantom::connect().await {
                Ok(pk) => wallet.set(Some(pk)),
                Err(e) => error.set(Some(e)),
            }
            busy.set(false);
        });
    };

    let on_claim_weekly = move |_| {
        let Some(addr) = wallet.get_untracked() else {
            return;
        };
        spawn_local(async move {
            busy.set(true);
            error.set(None);
            match api_post("/v1/rp/claim-weekly", &WeeklyClaimRequest { wallet: &addr }).await {
                Ok(resp) if resp.status().is_success() => {
                    notice.set(Some("Weekly promo RP claimed — free bucket, non-cashable.".into()));
                    refresh_rp(&addr, rp, &error).await;
                }
                Ok(resp) if resp.status().as_u16() == 429 => {
                    notice.set(Some("Already claimed this week.".into()));
                }
                Ok(resp) => error.set(Some(format!("claim failed: {resp:?}"))),
                Err(e) => error.set(Some(e)),
            }
            busy.set(false);
        });
    };

    view! {
        <main class="app">
            <header>
                <h1>"auctioning.lol"</h1>
                <Show
                    when=move || wallet.get().is_some()
                    fallback=view! {
                        <button class="btn-connect" on:click=on_connect disabled=busy>
                            {move || if busy.get() { "Connecting…" } else { "Connect Phantom" }}
                        </button>
                    }
                >
                    <span class="wallet-pill">
                        {move || truncate_wallet(wallet.get().unwrap_or_default())}
                    </span>
                </Show>
            </header>

            <Show when=move || error.get().is_some()>
                <p class="error">{move || error.get().unwrap_or_default()}</p>
            </Show>
            <Show when=move || notice.get().is_some()>
                <p class="notice">{move || notice.get().unwrap_or_default()}</p>
            </Show>

            <section class="rp-panel">
                <h2>"Your RP"</h2>
                <Suspense fallback=view! { <p>"Loading…"</p> }>
                    {move || Suspend::new(async move {
                        match rp.get() {
                            None => view! { <p>"Connect your wallet to see your balance."</p> }.into_any(),
                            Some(v) => view! {
                                <div class="rp-grid">
                                    <div><b>{v.paid_rp}</b><span>" paid RP (on-chain provenance)"</span></div>
                                    <div><b>{v.free_rp}</b><span>" free RP (non-cashable, expires)"</span></div>
                                    <div><b>{v.spent_rp}</b><span>" spent supporting projects"</span></div>
                                </div>
                                <button class="btn-claim" on:click=on_claim_weekly disabled=busy>
                                    "Claim weekly free RP"
                                </button>
                            }.into_any(),
                        }
                    })}
                </Suspense>
            </section>

            <LiveGrid />
            <RaceTape />
            <ProjectBoard wallet=wallet rp=rp />
            <Web3Actions wallet=wallet />
        </main>
    }
}

/// Live race grid derived from the private allocation ledger.
#[component]
fn LiveGrid() -> impl IntoView {
    let slots = RwSignal::new(Vec::<GridSlot>::new());
    let load_error = RwSignal::new(None::<String>);
    let busy = RwSignal::new(false);

    Effect::new(move |_| {
        spawn_local(async move {
            fetch_live_grid(slots, load_error, busy).await;
        });
    });

    view! {
        <section class="live-grid">
            <header class="grid-head">
                <h2>"Live Grid"</h2>
                <button class="btn-claim" on:click=move |_| spawn_local(async move { fetch_live_grid(slots, load_error, busy).await; }) disabled=busy>
                    {move || if busy.get() { "Refreshing…" } else { "Refresh" }}
                </button>
            </header>
            <p class="hint">"Ranks, velocity and gaps are derived from the allocation ledger. Free RP never goes on-chain."</p>
            <Show when=move || load_error.get().is_some()>
                <p class="error">{move || load_error.get().unwrap_or_default()}</p>
            </Show>
            <Show
                when=move || !slots.get().is_empty()
                fallback=view! { <p class="hint">"No race fuel yet — support a project to put it on the grid."</p> }
            >
                <table>
                    <thead>
                        <tr>
                            <th>"P"</th>
                            <th>"Project"</th>
                            <th>"Race RP"</th>
                            <th>"Vel"</th>
                            <th>"Mom"</th>
                            <th>"Gap"</th>
                        </tr>
                    </thead>
                    <tbody>
                        <For each=move || slots.get() key=|s| s.handle.clone() let:child>
                            <tr>
                                <td class="pos">{child.rank}</td>
                                <td>{child.handle.clone()}</td>
                                <td>{child.race_rp}</td>
                                <td>{child.velocity}</td>
                                <td>{child.momentum}</td>
                                <td>{child.gap_to_leader}</td>
                            </tr>
                        </For>
                    </tbody>
                </table>
            </Show>
        </section>
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TapePost {
    pub channel: String,
    pub body: String,
    pub source: String,
}

/// SLICE A: shareable race copy (templates; LLM is optional server-side).
#[component]
fn RaceTape() -> impl IntoView {
    let posts = RwSignal::new(Vec::<TapePost>::new());
    let load_error = RwSignal::new(None::<String>);

    Effect::new(move |_| {
        spawn_local(async move {
            let slug = match api_get("/v1/races/windows").await {
                Ok(resp) if resp.status().is_success() => {
                    #[derive(serde::Deserialize)]
                    struct Windows {
                        windows: Vec<WindowRow>,
                    }
                    #[derive(serde::Deserialize)]
                    struct WindowRow {
                        slug: String,
                    }
                    resp.json::<Windows>()
                        .await
                        .ok()
                        .and_then(|w| w.windows.into_iter().next().map(|x| x.slug))
                }
                _ => None,
            };
            let Some(slug) = slug else {
                return;
            };
            match api_get(&format!("/v1/races/windows/{slug}/tape")).await {
                Ok(resp) if resp.status().is_success() => {
                    #[derive(serde::Deserialize)]
                    struct Tape {
                        posts: Vec<TapePost>,
                    }
                    if let Ok(t) = resp.json::<Tape>().await {
                        posts.set(t.posts);
                    }
                }
                Ok(resp) => load_error.set(Some(format!("tape unavailable ({resp:?})"))),
                Err(e) => load_error.set(Some(e)),
            }
        });
    });

    view! {
        <section class="live-grid">
            <h2>"Race tape"</h2>
            <p class="hint">"Templated posts from race events. Why a project moved, not just that it moved."</p>
            <Show when=move || load_error.get().is_some()>
                <p class="error">{move || load_error.get().unwrap_or_default()}</p>
            </Show>
            <Show
                when=move || !posts.get().is_empty()
                fallback=view! { <p class="hint">"No posts yet — snapshot a live window, then narrate an event."</p> }
            >
                <ul>
                    <For each=move || posts.get() key=|p| format!("{}:{}", p.channel, p.body) let:child>
                        <li class="project-row">
                            <span class="pos">{child.channel.clone()}</span>
                            <span>{child.body.clone()}</span>
                        </li>
                    </For>
                </ul>
            </Show>
        </section>
    }
}

/// Ranked project board with a support action per project.
#[component]
fn ProjectBoard(wallet: RwSignal<Option<String>>, rp: RwSignal<Option<RpView>>) -> impl IntoView {
    let projects = RwSignal::new(Vec::<ProjectRow>::new());
    let load_error = RwSignal::new(None::<String>);

    Effect::new(move |_| {
        spawn_local(async move {
            match api_get("/v1/projects").await {
                Ok(resp) if resp.status().is_success() => {
                    #[derive(serde::Deserialize)]
                    struct Board {
                        projects: Vec<ProjectRow>,
                    }
                    if let Ok(b) = resp.json::<Board>().await {
                        projects.set(b.projects);
                    }
                }
                Ok(resp) => load_error.set(Some(format!("board unavailable ({resp:?})"))),
                Err(e) => load_error.set(Some(e)),
            }
        });
    });

    let support = move |handle: String| {
        let Some(_addr) = wallet.get_untracked() else {
            return;
        };
        spawn_local(async move {
            // v1 flow: support 25 free RP per click; a slider comes later.
            let req = SpendRequest { wallet: &_addr, amount: 25, reason: "support-project" };
            let path = format!("/v1/projects/{handle}/support");
            match api_post(&path, &req).await {
                Ok(resp) if resp.status().is_success() => {
                    refresh_rp(&_addr.clone(), rp, &load_error).await;
                    // Re-pull board totals lazily on next render cycle.
                }
                Ok(resp) if resp.status().as_u16() == 409 => {
                    load_error.set(Some("Not enough RP for that.".into()));
                }
                Ok(resp) => load_error.set(Some(format!("support failed: {resp:?}"))),
                Err(e) => load_error.set(Some(e)),
            }
        });
    };

    view! {
        <section class="board">
            <h2>"Project Board"</h2>
            <Show when=move || load_error.get().is_some()>
                <p class="error">{move || load_error.get().unwrap_or_default()}</p>
            </Show>
            <ul>
                <For each=move || projects.get() key=|p| p.handle.clone() let:child>
                    <li class="project-row">
                        <a class="project-name" href={format!("/p/{}", child.handle)}>
                            {child.display_name.unwrap_or_else(|| child.handle.clone())}
                        </a>
                        <span class="project-rp">{child.total_rp}" RP"</span>
                        <button on:click=move |_| support(child.handle.clone()) disabled=move || wallet.get().is_none()>
                            "Support +25"
                        </button>
                    </li>
                </For>
            </ul>
        </section>
    }
}

async fn fetch_live_grid(
    slots: RwSignal<Vec<GridSlot>>,
    load_error: RwSignal<Option<String>>,
    busy: RwSignal<bool>,
) {
    busy.set(true);
    match api_get("/v1/grid").await {
        Ok(resp) if resp.status().is_success() => {
            #[derive(serde::Deserialize)]
            struct Grid {
                grid: Vec<GridSlot>,
            }
            match resp.json::<Grid>().await {
                Ok(g) => {
                    slots.set(g.grid);
                    load_error.set(None);
                }
                Err(e) => load_error.set(Some(format!("grid parse: {e}"))),
            }
        }
        Ok(resp) => load_error.set(Some(format!("grid unavailable ({resp:?})"))),
        Err(e) => load_error.set(Some(e)),
    }
    busy.set(false);
}

async fn refresh_rp(addr: &str, rp: RwSignal<Option<RpView>>, err: &RwSignal<Option<String>>) {
    match api_get(&format!("/v1/rp/{addr}")).await {
        Ok(resp) if resp.status().is_success() => {
            if let Ok(v) = resp.json::<RpView>().await {
                rp.set(Some(v));
            }
        }
        Ok(resp) => err.set(Some(format!("refresh failed: {resp:?}"))),
        Err(e) => err.set(Some(e)),
    }
}

fn truncate_wallet(w: String) -> String {
    if w.len() > 12 {
        format!("{}…{}", &w[..6], &w[w.len() - 4..])
    } else {
        w
    }
}

/// On-chain actions + Whop purchase flow.
#[component]
fn Web3Actions(wallet: RwSignal<Option<String>>) -> impl IntoView {
    let busy = RwSignal::new(false);
    let notice = RwSignal::new(None::<String>);
    let handle = RwSignal::new(String::new());
    let log_rp = RwSignal::new(0u64);
    let log_lamports = RwSignal::new(0u64);
    let log_memo = RwSignal::new(String::new());
    let log_seq = RwSignal::new(0u64);
    let membership = RwSignal::new(None::<bool>);
    let race_pda = RwSignal::new(String::new());
    let race_nonce = RwSignal::new(0u64);
    let races = RwSignal::new(Vec::<serde_json::Value>::new());
    let settle_race_id = RwSignal::new(0u64);
    let current_program_id = RwSignal::new(String::new());

    let on_register_onchain = move |_| {
        let Some(addr) = wallet.get_untracked() else { return; };
        let h = handle.get_untracked();
        if h.trim().is_empty() { return; }

        spawn_local(async move {
            busy.set(true);
            notice.set(None);
            #[derive(serde::Serialize)]
            struct PrepReq { wallet: String, handle: String }
            let prep_req = PrepReq { wallet: addr.clone(), handle: h.clone() };
            match api_post("/v1/onchain/prepare-register", &prep_req).await {
                Ok(resp) if resp.status().is_success() => {
                    if let Ok(prep) = resp.json::<serde_json::Value>().await {
                        let tx_b64 = prep.get("tx_base64").and_then(|v| v.as_str()).unwrap_or("").to_string();
                        if tx_b64.is_empty() {
                            notice.set(Some("backend did not return tx".into()));
                        } else {
                            match Phantom::send_transaction(&addr, &tx_b64).await {
                                Ok(sig) => {
                                    let pda = prep.get("project_pda").and_then(|v| v.as_str()).unwrap_or("");
                                    let prog = prep.get("program_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                    current_program_id.set(prog.clone());
                                    let tx_url = format!("https://explorer.solana.com/tx/{}?cluster=mainnet", sig);
                                    let pda_url = if !pda.is_empty() { format!(" https://explorer.solana.com/address/{}?cluster=mainnet", pda) } else { "".to_string() };
                                    notice.set(Some(format!("Registered! sig:{}... TX:{} PDA:{}{} Program: {}", &sig[..12], tx_url, pda, pda_url, prog)));
                                    handle.set(String::new());
                                }
                                Err(e) => notice.set(Some(format!("send failed: {}", e))),
                            }
                        }
                    }
                }
                Ok(resp) => notice.set(Some(format!("prepare failed: status {}", resp.status()))),
                Err(e) => notice.set(Some(format!("network: {}", e))),
            }
            busy.set(false);
        });
    };

    let on_log_paid = move |_| {
        let Some(addr) = wallet.get_untracked() else { return; };
        let amt = log_rp.get_untracked();
        let lam = log_lamports.get_untracked();
        let m = log_memo.get_untracked();
        let seq = log_seq.get_untracked();
        if amt == 0 || lam == 0 || m.trim().is_empty() { return; }
        spawn_local(async move {
            busy.set(true);
            notice.set(None);
            #[derive(serde::Serialize)]
            struct LogReq { wallet: String, rp_amount: u64, lamports_paid: u64, memo: String, current_receipt_count: u64 }
            let req = LogReq { wallet: addr.clone(), rp_amount: amt, lamports_paid: lam, memo: m.clone(), current_receipt_count: seq };
            match api_post("/v1/onchain/prepare-log-paid", &req).await {
                Ok(resp) if resp.status().is_success() => {
                    if let Ok(prep) = resp.json::<serde_json::Value>().await {
                        let tx_b64 = prep.get("tx_base64").and_then(|v| v.as_str()).unwrap_or("").to_string();
                        if tx_b64.is_empty() {
                            notice.set(Some("no tx from backend".into()));
                        } else {
                            match Phantom::send_transaction(&addr, &tx_b64).await {
                                Ok(sig) => {
                                    let pda = prep.get("project_pda").and_then(|v| v.as_str()).unwrap_or("");
                                    let receipt = prep.get("receipt_pda").and_then(|v| v.as_str()).unwrap_or("");
                                    let prog = prep.get("program_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                    current_program_id.set(prog.clone());
                                    let tx_url = format!("https://explorer.solana.com/tx/{}?cluster=mainnet", sig);
                                    let receipt_url = if !receipt.is_empty() { format!(" https://explorer.solana.com/address/{}?cluster=mainnet", receipt) } else { "".to_string() };
                                    notice.set(Some(format!("Paid RP logged! sig:{}... TX:{} Receipt:{}{} Program: {}", &sig[..12], tx_url, receipt, receipt_url, prog)));
                                    log_rp.set(0); log_lamports.set(0); log_memo.set(String::new());
                                }
                                Err(e) => notice.set(Some(format!("send err: {}", e))),
                            }
                        }
                    }
                }
                _ => notice.set(Some("prepare failed".into())),
            }
            busy.set(false);
        });
    };

    let list_races = move |_| {
        let pda = race_pda.get_untracked();
        if pda.trim().is_empty() { return; }
        spawn_local(async move {
            busy.set(true);
            match api_get(&format!("/v1/races/{}", pda)).await {
                Ok(resp) if resp.status().is_success() => {
                    if let Ok(v) = resp.json::<serde_json::Value>().await {
                        if let Some(arr) = v.get("races").and_then(|r| r.as_array()) {
                            races.set(arr.clone());
                            notice.set(Some(format!("Loaded {} races for PDA", arr.len())));
                        }
                    }
                }
                _ => notice.set(Some("Failed to list races".into())),
            }
            busy.set(false);
        });
    };

    let on_open_race = move |_| {
        let Some(addr) = wallet.get_untracked() else { return; };
        let pda = race_pda.get_untracked();
        let nonce = race_nonce.get_untracked();
        if pda.trim().is_empty() { return; }
        spawn_local(async move {
            busy.set(true);
            #[derive(serde::Serialize)]
            struct OpenReq { wallet: String, project_pda: String, current_race_nonce: u64 }
            let req = OpenReq { wallet: addr.clone(), project_pda: pda.clone(), current_race_nonce: nonce };
            match api_post("/v1/onchain/prepare-open-race", &req).await {
                Ok(resp) if resp.status().is_success() => {
                    if let Ok(prep) = resp.json::<serde_json::Value>().await {
                        let tx_b64 = prep.get("tx_base64").and_then(|v| v.as_str()).unwrap_or("").to_string();
                        if tx_b64.is_empty() {
                            notice.set(Some("no tx".into()));
                        } else {
                            match Phantom::send_transaction(&addr, &tx_b64).await {
                                Ok(sig) => {
                                    let prog = prep.get("program_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                    current_program_id.set(prog.clone());
                                    notice.set(Some(format!("Race opened on L2! sig {}... Program: {}", &sig[..12], prog)));
                                }
                                Err(e) => notice.set(Some(format!("send err: {}", e))),
                            }
                        }
                    }
                }
                _ => notice.set(Some("prepare open race failed".into())),
            }
            busy.set(false);
        });
    };

    let on_settle_race = move |_| {
        let Some(addr) = wallet.get_untracked() else { return; };
        let pda = race_pda.get_untracked();
        let race_id = settle_race_id.get_untracked();
        if pda.trim().is_empty() { return; }
        spawn_local(async move {
            busy.set(true);
            // Demo: single sample result using the connected wallet as entrant
            #[derive(serde::Serialize)]
            struct Res { entrant: String, score: u64, rank: u16 }
            #[derive(serde::Serialize)]
            struct SettleReq { wallet: String, project_pda: String, race_id: u64, results: Vec<Res> }
            let sample = vec![Res { entrant: addr.clone(), score: 100, rank: 0 }];
            let req = SettleReq { wallet: addr.clone(), project_pda: pda.clone(), race_id, results: sample };
            match api_post("/v1/onchain/prepare-settle-race", &req).await {
                Ok(resp) if resp.status().is_success() => {
                    if let Ok(prep) = resp.json::<serde_json::Value>().await {
                        let tx_b64 = prep.get("tx_base64").and_then(|v| v.as_str()).unwrap_or("").to_string();
                        if tx_b64.is_empty() {
                            notice.set(Some("no tx".into()));
                        } else {
                            match Phantom::send_transaction(&addr, &tx_b64).await {
                                Ok(sig) => {
                                    let prog = prep.get("program_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                    current_program_id.set(prog.clone());
                                    notice.set(Some(format!("Race settled on-chain! sig {}... Program: {}", &sig[..12], prog)));
                                }
                                Err(e) => notice.set(Some(format!("send err: {}", e))),
                            }
                        }
                    }
                }
                _ => notice.set(Some("prepare settle failed".into())),
            }
            busy.set(false);
        });
    };

    let on_buy_rp = move |_| {
        let url = "https://whop.com/auctioning-rp";
        let _ = js_sys::eval(&format!("window.open('{}', '_blank')", url));
        notice.set(Some("Complete purchase on Whop. Paid RP will appear after webhook confirmation + optional on-chain receipt.".into()));
    };

    let check_whop = move |_| {
        let Some(addr) = wallet.get_untracked() else { return; };
        spawn_local(async move {
            busy.set(true);
            match api_get(&format!("/v1/whop/membership/{}", addr)).await {
                Ok(resp) if resp.status().is_success() => {
                    if let Ok(v) = resp.json::<serde_json::Value>().await {
                        let active = v.get("active_membership").and_then(|b| b.as_bool()).unwrap_or(false);
                        membership.set(Some(active));
                        notice.set(Some(format!("Whop: {} for {}", if active { "ACTIVE membership" } else { "no active membership" }, &addr[..8])));
                    }
                }
                _ => notice.set(Some("Whop check failed (backend or key)".into())),
            }
            busy.set(false);
        });
    };

    view! {
        <section class="web3-actions">
            <h2>"Web3 & Purchases"</h2>
            <div class="actions">
                <input
                    type="text"
                    placeholder="project-handle (e.g. my-project)"
                    prop:value=move || handle.get()
                    on:input=move |ev| handle.set(event_target_value(&ev))
                    disabled=move || busy.get()
                />
                <button
                    on:click=on_register_onchain
                    disabled=move || wallet.get().is_none() || busy.get() || handle.get().trim().is_empty()
                >
                    "Register Project On-Chain (Solana)"
                </button>
                <div class="log-paid">
                    <input type="number" placeholder="rp amt" prop:value=move || log_rp.get() on:input=move |ev| { if let Ok(v)=event_target_value(&ev).parse() { log_rp.set(v); } } disabled=busy />
                    <input type="number" placeholder="lamports" prop:value=move || log_lamports.get() on:input=move |ev| { if let Ok(v)=event_target_value(&ev).parse() { log_lamports.set(v); } } disabled=busy />
                    <input type="text" placeholder="memo" prop:value=move || log_memo.get() on:input=move |ev| log_memo.set(event_target_value(&ev)) disabled=busy />
                    <input type="number" placeholder="seq" prop:value=move || log_seq.get() on:input=move |ev| { if let Ok(v)=event_target_value(&ev).parse() { log_seq.set(v); } } disabled=busy />
                    <button on:click=on_log_paid disabled=move || wallet.get().is_none() || busy.get() || log_rp.get()==0 || log_lamports.get()==0 || log_memo.get().trim().is_empty() >
                        "Log Paid RP (Solana)"
                    </button>
                </div>
                <button on:click=on_buy_rp>
                    "Buy RP via Whop (fiat + Solana)"
                </button>
                <button on:click=check_whop disabled=move || wallet.get().is_none() || busy.get()>
                    "Check Whop Membership"
                </button>
                <div class="races">
                    <input type="text" placeholder="project_pda (base58)" prop:value=move || race_pda.get() on:input=move |ev| race_pda.set(event_target_value(&ev)) disabled=busy />
                    <input type="number" placeholder="current nonce" prop:value=move || race_nonce.get() on:input=move |ev| { if let Ok(v)=event_target_value(&ev).parse() { race_nonce.set(v); } } disabled=busy />
                    <button on:click=list_races disabled=move || race_pda.get().trim().is_empty() || busy.get()>
                        "List L2 Races (MagicBlock)"
                    </button>
                    <button on:click=on_open_race disabled=move || race_pda.get().trim().is_empty() || busy.get()>
                        "Open Race (Solana L2)"
                    </button>
                    <input type="number" placeholder="race id to settle" prop:value=move || settle_race_id.get() on:input=move |ev| { if let Ok(v)=event_target_value(&ev).parse() { settle_race_id.set(v); } } disabled=busy />
                    <button on:click=on_settle_race disabled=move || race_pda.get().trim().is_empty() || busy.get()>
                        "Settle Race (on-chain)"
                    </button>
                    <Show when=move || !races.get().is_empty()>
                        <ul>
                            <For each=move || races.get() key=|r| r.get("id").and_then(|i| i.as_i64()).unwrap_or(0) let:child>
                                <li>
                                    {let c = child.clone(); move || format!("race {} status: {}", c.get("race_id").and_then(|v| v.as_i64()).unwrap_or(0), c.get("status").and_then(|v| v.as_str()).unwrap_or("")) }
                                    <button on:click={
                                        let c = child.clone();
                                        move |_| {
                                            if let Some(rid) = c.get("race_id").and_then(|v| v.as_i64()) {
                                                settle_race_id.set(rid as u64);
                                            }
                                        }
                                    }>"Settle this"</button>
                                </li>
                            </For>
                        </ul>
                    </Show>
                </div>
            </div>
            <p class="hint">"Register (on-chain project) or log paid RP receipt via Phantom + Anchor for Solana provenance. (L2 races via MagicBlock.)"</p>
            <Show when=move || !current_program_id.get().is_empty()>
                <p class="onchain-program">"On-chain Program: " {move || current_program_id.get()} 
                <a href=move || format!("https://explorer.solana.com/address/{}?cluster=mainnet", current_program_id.get()) target="_blank">" (view on explorer)"</a> " (set PROGRAM_ID secret for real mainnet deploy)"</p>
            </Show>
            <Show when=move || notice.get().is_some()>
                <p class="notice">{move || notice.get().unwrap_or_default()}</p>
            </Show>
        </section>
    }
}

/// Mount the CSR app into `#root`. Called from main.rs.
pub fn mount_app() {
    mount_to_body(|| view! { <App /> });
}