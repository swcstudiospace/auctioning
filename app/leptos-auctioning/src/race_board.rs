//! Race-day board: one ranking at a time, hover/sheet card, secondary rail.
//!
//! RaceTape stays in `App` — this module does not duplicate the ticker.

use crate::{api_get, GridSlot};
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;
use std::time::Duration;

const HOVER_DELAY: Duration = Duration::from_millis(250);
const SEASON_LABEL: &str = "2026 Season 1";

#[derive(Clone, Copy, PartialEq, Eq)]
enum BoardTab {
    LiveGrid,
    Sprints,
    GrandPrix,
    Championship,
    Specials,
}

impl BoardTab {
    fn label(self) -> &'static str {
        match self {
            BoardTab::LiveGrid => "Live Grid",
            BoardTab::Sprints => "Sprints",
            BoardTab::GrandPrix => "Grand Prix",
            BoardTab::Championship => "Championship",
            BoardTab::Specials => "Specials",
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
struct RaceWindow {
    #[serde(default)]
    slug: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    race_type: String,
    #[serde(default)]
    status: String,
    #[serde(default)]
    starts_at: String,
    #[serde(default)]
    ends_at: String,
}

#[derive(Clone, Debug, Deserialize)]
struct ChampStanding {
    #[serde(default)]
    handle: String,
    #[serde(default)]
    points: i64,
    #[serde(default)]
    wins: i64,
    #[serde(default)]
    best_finish: i32,
}

#[derive(Clone, Debug, Deserialize)]
struct ActiveCard {
    #[serde(default)]
    name: String,
}

fn type_key(t: &str) -> String {
    t.trim().to_ascii_uppercase()
}

fn is_open(status: &str) -> bool {
    status.eq_ignore_ascii_case("live") || status.eq_ignore_ascii_case("scheduled")
}

fn is_sprint(t: &str) -> bool {
    matches!(type_key(t).as_str(), "GREEN_FLAG" | "SPRINT" | "PACE_LAP")
}

fn is_gp(t: &str) -> bool {
    matches!(
        type_key(t).as_str(),
        "GRAND_TOUR" | "GRAND_PRIX" | "SECTOR_SCRAP"
    )
}

fn is_special(t: &str) -> bool {
    matches!(type_key(t).as_str(), "SPECIAL_EVENT" | "PHOTO_CARD")
}

fn is_live(status: &str) -> bool {
    status.eq_ignore_ascii_case("live")
}

fn parse_ms(iso: &str) -> Option<f64> {
    if iso.is_empty() {
        return None;
    }
    let t = js_sys::Date::parse(iso);
    if t.is_nan() {
        None
    } else {
        Some(t)
    }
}

fn format_countdown(ends_at: &str, now_ms: f64) -> Option<String> {
    let end = parse_ms(ends_at)?;
    let rem = ((end - now_ms) / 1000.0).floor() as i64;
    if rem <= 0 {
        return Some("ended".into());
    }
    let h = rem / 3600;
    let m = (rem % 3600) / 60;
    let s = rem % 60;
    if h > 0 {
        Some(format!("{h}h {m:02}m"))
    } else {
        Some(format!("{m}m {s:02}s"))
    }
}

fn grand_tour_day(starts_at: &str, ends_at: &str, now_ms: f64) -> Option<String> {
    if starts_at.is_empty() || ends_at.is_empty() {
        return None;
    }
    let start = parse_ms(starts_at)?;
    let elapsed = now_ms - start;
    let day = ((elapsed / 86_400_000.0).floor() as i32) + 1;
    Some(format!("Day {}/7", day.clamp(1, 7)))
}

fn gap_display(gap: Option<i64>) -> String {
    gap.map(|n| n.to_string()).unwrap_or_else(|| "—".into())
}

fn progress_target(row: &GridSlot) -> i64 {
    row.race_rp.saturating_add(row.gap_to_next.unwrap_or(0))
}

fn progress_fill_pct(race_rp: i64, target: i64) -> i64 {
    if target > 0 {
        race_rp.saturating_mul(100) / target
    } else {
        100
    }
}

fn mix_line(paid_rp: i64, community_rp: i64) -> String {
    let total = paid_rp.saturating_add(community_rp);
    if total <= 0 {
        "—".into()
    } else {
        let paid_pct = paid_rp.saturating_mul(100) / total;
        let community_pct = community_rp.saturating_mul(100) / total;
        format!("{paid_pct}% paid / {community_pct}% community")
    }
}

fn avatar_tone(handle: &str) -> &'static str {
    const TONES: [&str; 5] = [
        "letter-avatar tone-0",
        "letter-avatar tone-1",
        "letter-avatar tone-2",
        "letter-avatar tone-3",
        "letter-avatar tone-4",
    ];
    let h = handle.as_bytes().iter().fold(0u32, |acc, b| {
        acc.wrapping_mul(31).wrapping_add(u32::from(*b))
    });
    TONES[h as usize % TONES.len()]
}

fn avatar_initial(handle: &str) -> String {
    handle
        .chars()
        .next()
        .map(|c| c.to_uppercase().collect::<String>())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "?".into())
}

fn lifetime_display(rank: Option<i32>) -> String {
    rank.map(|r| format!("P{r}")).unwrap_or_else(|| "—".into())
}

fn cpc_display(row: &GridSlot) -> String {
    if row.clicks == 0 {
        "—".into()
    } else {
        format!("{}", row.race_rp / row.clicks)
    }
}

fn badge_class(badge: &str) -> &'static str {
    match badge {
        "HOT" => "badge-chip badge-hot",
        "PHOTO" => "badge-chip badge-photo",
        "DARK_HORSE" => "badge-chip badge-dark-horse",
        "REIGN" => "badge-chip badge-reign",
        "COOLING" => "badge-chip badge-cooling",
        _ => "badge-chip",
    }
}

fn badge_label(badge: &str) -> String {
    badge.replace('_', " ")
}

#[component]
fn LetterAvatar(handle: String) -> impl IntoView {
    let class = avatar_tone(&handle);
    let initial = avatar_initial(&handle);
    view! {
        <span class=class aria-hidden="true">{initial}</span>
    }
}

fn go_garage(handle: &str) {
    if let Some(w) = web_sys::window() {
        let _ = w.location().set_href(&format!("/p/{handle}"));
    }
}

fn extract_because(v: &serde_json::Value) -> Option<String> {
    v.get("because")
        .and_then(|x| x.as_str())
        .or_else(|| {
            v.get("featured")
                .and_then(|f| f.get("because"))
                .and_then(|x| x.as_str())
        })
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn pick_sprint(windows: &[RaceWindow]) -> Option<&RaceWindow> {
    windows
        .iter()
        .find(|w| is_sprint(&w.race_type) && is_live(&w.status))
        .or_else(|| {
            windows
                .iter()
                .find(|w| is_sprint(&w.race_type) && is_open(&w.status))
        })
}

fn pick_grand_tour(windows: &[RaceWindow]) -> Option<&RaceWindow> {
    windows
        .iter()
        .find(|w| is_gp(&w.race_type) && is_live(&w.status))
        .or_else(|| {
            windows
                .iter()
                .find(|w| is_gp(&w.race_type) && is_open(&w.status))
        })
}

fn pick_live_gp(windows: &[RaceWindow]) -> Option<&RaceWindow> {
    windows
        .iter()
        .find(|w| is_live(&w.status) && is_gp(&w.race_type))
}

fn clear_timer(timer: RwSignal<Option<TimeoutHandle>>) {
    if let Some(h) = timer.get_untracked() {
        h.clear();
        timer.set(None);
    }
}

async fn load_grid(
    slots: RwSignal<Vec<GridSlot>>,
    load_error: RwSignal<Option<String>>,
    busy: RwSignal<bool>,
) {
    busy.set(true);
    match api_get("/v1/grid").await {
        Ok(resp) if resp.status().is_success() => {
            #[derive(Deserialize)]
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

/// Homepage race board. One RP ranking visible at a time.
#[component]
pub fn RaceBoard() -> impl IntoView {
    let tab = RwSignal::new(BoardTab::LiveGrid);
    let live_slots = RwSignal::new(Vec::<GridSlot>::new());
    let gp_slots = RwSignal::new(Vec::<GridSlot>::new());
    let gp_fallback = RwSignal::new(true);
    let load_error = RwSignal::new(None::<String>);
    let busy = RwSignal::new(false);
    let windows = RwSignal::new(Vec::<RaceWindow>::new());
    let card_name = RwSignal::new(None::<String>);
    let champ = RwSignal::new(Vec::<ChampStanding>::new());
    let champ_ok = RwSignal::new(false);
    let featured = RwSignal::new(None::<String>);
    let selected = RwSignal::new(None::<String>);
    let selected_row = RwSignal::new(None::<GridSlot>);
    let opened_by_tap = RwSignal::new(false);
    let touch_ui = RwSignal::new(false);
    let hover_timer = RwSignal::new(None::<TimeoutHandle>);
    let now_ms = RwSignal::new(js_sys::Date::now());

    Effect::new(move |_| {
        spawn_local(async move {
            load_grid(live_slots, load_error, busy).await;
        });
        spawn_local(async move {
            match api_get("/v1/races/windows").await {
                Ok(resp) if resp.status().is_success() => {
                    #[derive(Deserialize)]
                    struct Windows {
                        #[serde(default)]
                        windows: Vec<RaceWindow>,
                    }
                    if let Ok(w) = resp.json::<Windows>().await {
                        windows.set(w.windows);
                    }
                }
                _ => {}
            }
        });
        spawn_local(async move {
            match api_get("/v1/events/active").await {
                Ok(resp) if resp.status().is_success() => {
                    #[derive(Deserialize)]
                    struct Wrap {
                        active: Option<ActiveCard>,
                    }
                    if let Ok(w) = resp.json::<Wrap>().await {
                        card_name.set(w.active.and_then(|c| {
                            if c.name.is_empty() {
                                None
                            } else {
                                Some(c.name)
                            }
                        }));
                    }
                }
                _ => {}
            }
        });
        spawn_local(async move {
            match api_get("/v1/championship").await {
                Ok(resp) if resp.status().is_success() => {
                    champ_ok.set(true);
                    match resp.json::<serde_json::Value>().await {
                        Ok(v) => {
                            let rows = v
                                .get("standings")
                                .cloned()
                                .and_then(|a| serde_json::from_value::<Vec<ChampStanding>>(a).ok())
                                .unwrap_or_default()
                                .into_iter()
                                .filter(|r| !r.handle.is_empty())
                                .collect();
                            champ.set(rows);
                        }
                        Err(_) => champ.set(Vec::new()),
                    }
                }
                _ => champ_ok.set(false),
            }
        });
        spawn_local(async move {
            match api_get("/v1/races/featured").await {
                Ok(resp) if resp.status().is_success() => {
                    if let Ok(v) = resp.json::<serde_json::Value>().await {
                        featured.set(extract_because(&v));
                    }
                }
                _ => featured.set(None),
            }
        });
    });

    Effect::new(move |_| {
        now_ms.set(js_sys::Date::now());
        if let Ok(handle) = set_interval_with_handle(
            move || now_ms.set(js_sys::Date::now()),
            Duration::from_secs(1),
        ) {
            on_cleanup(move || handle.clear());
        }
    });

    Effect::new(move |_| {
        let current = tab.get();
        let list = windows.get();
        if current != BoardTab::GrandPrix {
            return;
        }
        let slug = pick_live_gp(&list)
            .map(|w| w.slug.clone())
            .filter(|s| !s.is_empty());
        spawn_local(async move {
            let Some(slug) = slug else {
                gp_fallback.set(true);
                return;
            };
            match api_get(&format!("/v1/races/windows/{slug}/grid")).await {
                Ok(resp) if resp.status().is_success() => {
                    #[derive(Deserialize)]
                    struct Grid {
                        #[serde(default)]
                        grid: Vec<GridSlot>,
                    }
                    match resp.json::<Grid>().await {
                        Ok(g) => {
                            gp_slots.set(g.grid);
                            gp_fallback.set(false);
                        }
                        Err(_) => gp_fallback.set(true),
                    }
                }
                _ => gp_fallback.set(true),
            }
        });
    });

    let on_touch = move |_| {
        touch_ui.set(true);
        clear_timer(hover_timer);
    };

    view! {
        <section class="race-board" on:touchstart=on_touch>
            <div class="race-main">
                <p class="season-label">{SEASON_LABEL}</p>
                <nav class="race-tabs" role="tablist" aria-label="Race board">
                    <TabButton tab=tab id=BoardTab::LiveGrid />
                    <TabButton tab=tab id=BoardTab::Sprints />
                    <TabButton tab=tab id=BoardTab::GrandPrix />
                    <TabButton tab=tab id=BoardTab::Championship />
                    <TabButton tab=tab id=BoardTab::Specials />
                </nav>

                <Show when=move || load_error.get().is_some() && tab.get() == BoardTab::LiveGrid>
                    <p class="error">{move || load_error.get().unwrap_or_default()}</p>
                </Show>

                {move || match tab.get() {
                    BoardTab::LiveGrid => view! {
                        <div class="grid-head">
                            <p class="hint">"P# · handle · race RP · progress. Rank is windowed RP."</p>
                            <button
                                class="btn-claim"
                                on:click=move |_| spawn_local(async move {
                                    load_grid(live_slots, load_error, busy).await;
                                })
                                disabled=busy
                            >
                                {move || if busy.get() { "Refreshing…" } else { "Refresh" }}
                            </button>
                        </div>
                        <RankRows
                            rows=live_slots
                            selected=selected
                            selected_row=selected_row
                            opened_by_tap=opened_by_tap
                            touch_ui=touch_ui
                            hover_timer=hover_timer
                        />
                    }.into_any(),
                    BoardTab::Sprints => view! { <SprintList windows=windows /> }.into_any(),
                    BoardTab::GrandPrix => {
                        let rows = if gp_fallback.get() { live_slots } else { gp_slots };
                        view! {
                            <p class="hint">
                                {if gp_fallback.get() {
                                    "No live Grand Tour — showing the Live Grid."
                                } else {
                                    "Grand Tour / Grand Prix window grid."
                                }}
                            </p>
                            <RankRows
                                rows=rows
                                selected=selected
                                selected_row=selected_row
                                opened_by_tap=opened_by_tap
                                touch_ui=touch_ui
                                hover_timer=hover_timer
                            />
                        }.into_any()
                    }
                    BoardTab::Championship => view! { <ChampTable standings=champ /> }.into_any(),
                    BoardTab::Specials => view! {
                        <SpecialsPanel card_name=card_name windows=windows />
                    }.into_any(),
                }}
            </div>

            <aside class="race-rail">
                <Rail
                    windows=windows
                    card_name=card_name
                    champ=champ
                    champ_ok=champ_ok
                    featured=featured
                    now_ms=now_ms
                />
            </aside>

            <Sheet
                selected_row=selected_row
                selected=selected
                opened_by_tap=opened_by_tap
            />
        </section>
    }
}

#[component]
fn TabButton(tab: RwSignal<BoardTab>, id: BoardTab) -> impl IntoView {
    view! {
        <button
            type="button"
            role="tab"
            class=move || {
                if tab.get() == id {
                    "race-tab is-active"
                } else {
                    "race-tab"
                }
            }
            aria-selected=move || tab.get() == id
            on:click=move |_| {
                tab.set(id);
            }
        >
            {id.label()}
        </button>
    }
}

#[component]
fn RankRows(
    rows: RwSignal<Vec<GridSlot>>,
    selected: RwSignal<Option<String>>,
    selected_row: RwSignal<Option<GridSlot>>,
    opened_by_tap: RwSignal<bool>,
    touch_ui: RwSignal<bool>,
    hover_timer: RwSignal<Option<TimeoutHandle>>,
) -> impl IntoView {
    view! {
        <Show
            when=move || !rows.get().is_empty()
            fallback=view! { <p class="hint">"No race fuel yet — support a project to put it on the grid."</p> }
        >
            <div class="race-rank" role="table">
                <div class="race-rank-head" role="row">
                    <span>"P"</span>
                    <span>"Project"</span>
                    <span>"Race RP"</span>
                    <span>"Progress"</span>
                    <span></span>
                </div>
                <For each=move || rows.get() key=|s| s.handle.clone() let:child>
                    <RankRow
                        row=child
                        selected=selected
                        selected_row=selected_row
                        opened_by_tap=opened_by_tap
                        touch_ui=touch_ui
                        hover_timer=hover_timer
                    />
                </For>
            </div>
        </Show>
    }
}

#[component]
fn RankRow(
    row: GridSlot,
    selected: RwSignal<Option<String>>,
    selected_row: RwSignal<Option<GridSlot>>,
    opened_by_tap: RwSignal<bool>,
    touch_ui: RwSignal<bool>,
    hover_timer: RwSignal<Option<TimeoutHandle>>,
) -> impl IntoView {
    let handle = row.handle.clone();
    let handle_enter = handle.clone();
    let handle_click = handle.clone();
    let handle_class = handle.clone();
    let row_enter = row.clone();
    let row_click = row.clone();
    let row_card = row.clone();
    let badge = row.badge.clone();
    let pos = format!("P{}", row.rank);
    let race_rp = row.race_rp;
    let target = progress_target(&row);
    let fill = progress_fill_pct(race_rp, target);
    let fill_style = format!("--fill:{fill}%");
    let progress_label = format!("{race_rp} / {target} RP");
    let progress_aria = progress_label.clone();
    let handle_label = row.handle.clone();
    let avatar_handle = handle_label.clone();

    view! {
        <div
            class=move || {
                if selected.get().as_deref() == Some(handle_class.as_str()) {
                    "race-row is-open"
                } else {
                    "race-row"
                }
            }
            role="row"
            on:mouseenter=move |_| {
                if touch_ui.get_untracked() {
                    return;
                }
                clear_timer(hover_timer);
                let handle = handle_enter.clone();
                let row = row_enter.clone();
                if let Ok(h) = set_timeout_with_handle(
                    move || {
                        selected.set(Some(handle));
                        selected_row.set(Some(row));
                        opened_by_tap.set(false);
                    },
                    HOVER_DELAY,
                ) {
                    hover_timer.set(Some(h));
                }
            }
            on:mouseleave=move |_| {
                if touch_ui.get_untracked() {
                    return;
                }
                clear_timer(hover_timer);
                selected.set(None);
                selected_row.set(None);
                opened_by_tap.set(false);
            }
        >
            <div
                class="race-row-main"
                on:click=move |_| {
                    if !touch_ui.get_untracked() {
                        return;
                    }
                    let same = selected.get_untracked().as_deref() == Some(handle_click.as_str());
                    if same && opened_by_tap.get_untracked() {
                        go_garage(&handle_click);
                    } else {
                        selected.set(Some(handle_click.clone()));
                        selected_row.set(Some(row_click.clone()));
                        opened_by_tap.set(true);
                    }
                }
            >
                <span class="pos">{pos}</span>
                <span class="race-driver">
                    <LetterAvatar handle=avatar_handle />
                    <span class="race-handle">{handle_label}</span>
                </span>
                <span class="race-rp">{race_rp}</span>
                <span class="race-progress" aria-label=progress_aria>
                    <span class="race-progress-track">
                        <span class="race-progress-fill" style=fill_style></span>
                    </span>
                    <span class="race-progress-label">{progress_label}</span>
                </span>
                <span class="race-badge">
                    {badge.map(|b| {
                        let class = badge_class(&b);
                        let label = badge_label(&b);
                        view! { <span class=class>{label}</span> }
                    })}
                </span>
            </div>
            <div class="race-hover">
                <HoverCard row=row_card />
            </div>
        </div>
    }
}

#[component]
fn HoverCard(row: GridSlot) -> impl IntoView {
    let garage = format!("/p/{}", row.handle);
    let mix = mix_line(row.paid_rp, row.community_rp);
    let last = row
        .last_overtake
        .clone()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "—".into());
    let footer = row.hover_footer.clone();
    let footer_class = if footer.is_empty() {
        "hover-footer"
    } else {
        "hover-footer has-flag"
    };
    let cpc = cpc_display(&row);
    let life = lifetime_display(row.lifetime_rank);
    let gap = gap_display(row.gap_to_next);
    let gap_class = if row.gap_to_next.is_some() {
        "hover-gap"
    } else {
        ""
    };
    let pos = format!("P{}", row.rank);
    let handle = row.handle.clone();
    let avatar_handle = row.handle.clone();
    let race_rp = row.race_rp;
    let clicks = row.clicks;

    view! {
        <div class="slot-card">
            <div class="hover-identity">
                <LetterAvatar handle=avatar_handle />
                <strong class="hover-handle">{handle}</strong>
            </div>
            <div class="hover-band">
                <p class="hover-band-label">"Racing telemetry"</p>
                <div class="hover-stats hover-telemetry">
                    <div class="hover-stat"><b>{pos}</b><span>"Position"</span></div>
                    <div class="hover-stat">
                        <b class=gap_class>{gap}</b>
                        <span>"GAP"</span>
                    </div>
                </div>
            </div>
            <div class="hover-band">
                <p class="hover-band-label">"Last overtake"</p>
                <p class="hover-overtake">{last}</p>
            </div>
            <div class="hover-band">
                <p class="hover-band-label">"Business intelligence"</p>
                <div class="hover-stats hover-bi">
                    <div class="hover-stat">
                        <b class="hover-rp">{race_rp}</b>
                        <span>"Race RP"</span>
                    </div>
                    <div class="hover-stat"><b>{life}</b><span>"Lifetime"</span></div>
                    <div class="hover-stat"><b>{clicks}</b><span>"Clicks"</span></div>
                    <div class="hover-stat">
                        <b>{cpc}</b>
                        <span>"CPC"</span>
                        <span class="hover-caption">"board clicks, unverified"</span>
                    </div>
                </div>
                <p class="hover-mix">{mix}</p>
            </div>
            <p class=footer_class>{footer}</p>
            <a class="btn-garage" href=garage>"Open garage"</a>
        </div>
    }
}

#[component]
fn Sheet(
    selected_row: RwSignal<Option<GridSlot>>,
    selected: RwSignal<Option<String>>,
    opened_by_tap: RwSignal<bool>,
) -> impl IntoView {
    let close = move |_| {
        selected.set(None);
        selected_row.set(None);
        opened_by_tap.set(false);
    };
    view! {
        <div
            class=move || {
                if selected_row.get().is_some() {
                    "race-sheet-backdrop is-open"
                } else {
                    "race-sheet-backdrop"
                }
            }
            on:click=close
        ></div>
        <div
            class=move || {
                if selected_row.get().is_some() {
                    "race-sheet is-open"
                } else {
                    "race-sheet"
                }
            }
        >
            <div class="sheet-handle"></div>
            {move || selected_row.get().map(|row| view! { <HoverCard row=row /> })}
        </div>
    }
}

#[component]
fn SprintList(windows: RwSignal<Vec<RaceWindow>>) -> impl IntoView {
    let sprint_rows = move || {
        windows
            .get()
            .into_iter()
            .filter(|w| is_sprint(&w.race_type))
            .collect::<Vec<_>>()
    };
    view! {
        <Show
            when=move || !sprint_rows().is_empty()
            fallback=view! { <p class="hint">"No Green Flag, Sprint, or Pace Lap windows on the board."</p> }
        >
            <ul class="race-list">
                <For each=sprint_rows key=|w| w.slug.clone() let:child>
                    <li class="race-list-row">
                        <span class="race-list-name">{child.name.clone()}</span>
                        <span class="hint">{child.status.clone()}</span>
                    </li>
                </For>
            </ul>
        </Show>
    }
}

#[component]
fn ChampTable(standings: RwSignal<Vec<ChampStanding>>) -> impl IntoView {
    view! {
        <p class="hint">"Championship is points, not RP."</p>
        <Show
            when=move || !standings.get().is_empty()
            fallback=view! { <p class="hint">"No championship standings yet."</p> }
        >
            <div class="champ-table" role="table">
                <div class="champ-head" role="row">
                    <span>"Handle"</span>
                    <span>"Points"</span>
                    <span>"Wins"</span>
                    <span>"Best finish"</span>
                </div>
                <For each=move || standings.get() key=|s| s.handle.clone() let:child>
                    <div class="champ-row" role="row">
                        <span>{child.handle.clone()}</span>
                        <span class="race-rp">{child.points}</span>
                        <span>{child.wins}</span>
                        <span>
                            {if child.best_finish > 0 {
                                format!("P{}", child.best_finish)
                            } else {
                                "—".into()
                            }}
                        </span>
                    </div>
                </For>
            </div>
        </Show>
    }
}

#[component]
fn SpecialsPanel(
    card_name: RwSignal<Option<String>>,
    windows: RwSignal<Vec<RaceWindow>>,
) -> impl IntoView {
    let specials = move || {
        windows
            .get()
            .into_iter()
            .filter(|w| is_special(&w.race_type))
            .collect::<Vec<_>>()
    };
    let empty = move || card_name.get().is_none() && specials().is_empty();
    view! {
        <Show
            when=move || !empty()
            fallback=view! { <p class="hint">"No operator card or special windows live."</p> }
        >
            <Show when=move || card_name.get().is_some()>
                <p class="special-card">
                    {move || card_name.get().unwrap_or_default()}
                    " — operator card"
                </p>
            </Show>
            <ul class="race-list">
                <For each=specials key=|w| w.slug.clone() let:child>
                    <li class="race-list-row">
                        <span class="race-list-name">{child.name.clone()}</span>
                        <span class="hint">{child.status.clone()}</span>
                    </li>
                </For>
            </ul>
        </Show>
    }
}

#[component]
fn Rail(
    windows: RwSignal<Vec<RaceWindow>>,
    card_name: RwSignal<Option<String>>,
    champ: RwSignal<Vec<ChampStanding>>,
    champ_ok: RwSignal<bool>,
    featured: RwSignal<Option<String>>,
    now_ms: RwSignal<f64>,
) -> impl IntoView {
    view! {
        <h2 class="rail-title">"Today's calendar"</h2>
        <div class="rail-track">
            <div class="rail-block">
                <h3>"Sprint"</h3>
                {move || {
                    let now = now_ms.get();
                    match pick_sprint(&windows.get()) {
                        Some(w) => {
                            let cd = format_countdown(&w.ends_at, now).unwrap_or_else(|| "—".into());
                            view! {
                                <p class="rail-value">{format!("{} · {}", w.name, cd)}</p>
                            }.into_any()
                        }
                        None => view! { <p class="hint">"No sprint clock"</p> }.into_any(),
                    }
                }}
            </div>
            <div class="rail-block">
                <h3>"Grand Tour"</h3>
                {move || {
                    let now = now_ms.get();
                    match pick_grand_tour(&windows.get()) {
                        Some(w) => match grand_tour_day(&w.starts_at, &w.ends_at, now) {
                            Some(day) => view! {
                                <p class="rail-value">{format!("{} · {}", w.name, day)}</p>
                            }.into_any(),
                            None => view! { <p class="hint">"Grand Tour window open"</p> }.into_any(),
                        },
                        None => view! { <p class="hint">"No Grand Tour window"</p> }.into_any(),
                    }
                }}
            </div>
            <div class="rail-block">
                <h3>"Championship"</h3>
                {move || {
                    if champ_ok.get() {
                        if let Some(lead) = champ.get().into_iter().next() {
                            view! {
                                <p class="rail-value">
                                    {format!("{} leads · {} pts", lead.handle, lead.points)}
                                </p>
                            }.into_any()
                        } else {
                            view! { <p class="hint">"Championship points pending"</p> }.into_any()
                        }
                    } else {
                        view! { <p class="hint">"Championship points pending"</p> }.into_any()
                    }
                }}
            </div>
            <div class="rail-block">
                <h3>"Card"</h3>
                {move || match card_name.get() {
                    Some(name) => view! { <p class="rail-value">{name}</p> }.into_any(),
                    None => view! { <p class="hint">"No operator card"</p> }.into_any(),
                }}
            </div>
            <Show when=move || featured.get().is_some()>
                <div class="rail-block">
                    <h3>"Featured"</h3>
                    <p class="featured-chip">{move || featured.get().unwrap_or_default()}</p>
                </div>
            </Show>
        </div>
    }
}
