//! Pure featured-race picker.
//!
//! Score is a 0..=10000 overlay on windowed RP rank — never purchasable
//! velocity. Each signal is clamped 0..=100 before weighting:
//! `25*O + 20*P + 15*U + 10*M + 10*T + 10*D + 10*A`.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct FeaturedSignals {
    pub window_slug: String,
    pub window_name: String,
    pub overtake_density: i64,      // O 0..=100
    pub photo_finish_pressure: i64, // P
    pub unique_payers: i64,         // U
    pub mix: i64,                   // M not 100% whale and not 100% community
    pub time_remaining: i64,        // T
    pub freshness: i64,             // D dark horse / new rivalry
    pub attention: i64,             // A
    pub overtakes_in_window: i64,   // raw count for because-line
    pub p1_p3_cover_rp: i64,        // RP covering P1–P3 for because-line
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeaturedRace {
    pub window_slug: String,
    pub window_name: String,
    pub score: i64, // 0..=10000
    pub because: String,
}

fn clamp100(x: i64) -> i64 {
    x.clamp(0, 100)
}

/// Weighted featured score for one window. Because-line uses the raw
/// overtake count and P1–P3 cover, not the clamped weights.
pub fn featured_score(s: &FeaturedSignals) -> FeaturedRace {
    let o = clamp100(s.overtake_density);
    let p = clamp100(s.photo_finish_pressure);
    let u = clamp100(s.unique_payers);
    let m = clamp100(s.mix);
    let t = clamp100(s.time_remaining);
    let d = clamp100(s.freshness);
    let a = clamp100(s.attention);
    let score = 25 * o + 20 * p + 15 * u + 10 * m + 10 * t + 10 * d + 10 * a;
    FeaturedRace {
        window_slug: s.window_slug.clone(),
        window_name: s.window_name.clone(),
        score,
        because: format!(
            "Featured because {} overtakes in window · {} RP cover P1–P3",
            s.overtakes_in_window, s.p1_p3_cover_rp
        ),
    }
}

/// Highest featured score wins. Ties keep the first candidate in the slice.
pub fn pick_featured(cands: &[FeaturedSignals]) -> Option<FeaturedRace> {
    cands
        .iter()
        .map(featured_score)
        .reduce(|best, next| if next.score > best.score { next } else { best })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn signals(
        slug: &str,
        name: &str,
        o: i64,
        p: i64,
        u: i64,
        m: i64,
        t: i64,
        d: i64,
        a: i64,
        overtakes: i64,
        cover: i64,
    ) -> FeaturedSignals {
        FeaturedSignals {
            window_slug: slug.to_string(),
            window_name: name.to_string(),
            overtake_density: o,
            photo_finish_pressure: p,
            unique_payers: u,
            mix: m,
            time_remaining: t,
            freshness: d,
            attention: a,
            overtakes_in_window: overtakes,
            p1_p3_cover_rp: cover,
        }
    }

    #[test]
    fn all_100_scores_10000() {
        let s = signals("grand_tour", "Grand Tour", 100, 100, 100, 100, 100, 100, 100, 12, 40);
        let r = featured_score(&s);
        assert_eq!(r.score, 10_000);
        assert_eq!(r.window_slug, "grand_tour");
        assert_eq!(r.window_name, "Grand Tour");
    }

    #[test]
    fn zeros_score_0() {
        let s = signals("pace_lap", "Pace Lap", 0, 0, 0, 0, 0, 0, 0, 0, 0);
        assert_eq!(featured_score(&s).score, 0);
    }

    #[test]
    fn overtake_only_is_2500() {
        let s = signals("green_flag", "Green Flag", 100, 0, 0, 0, 0, 0, 0, 7, 0);
        assert_eq!(featured_score(&s).score, 2500);
    }

    #[test]
    fn clamp_keeps_score_in_0_10000() {
        let over = signals("gt", "Grand Tour", 150, 200, 130, 101, 999, 100, 110, 1, 1);
        assert_eq!(featured_score(&over).score, 10_000);
        let under = signals("gt", "Grand Tour", -1, -20, -5, -9, -1, -2, -3, 0, 0);
        assert_eq!(featured_score(&under).score, 0);
    }

    #[test]
    fn because_uses_raw_overtakes_and_cover_not_weights() {
        let s = signals("sector_scrap", "Sector Scrap", 100, 0, 0, 0, 0, 0, 0, 17, 83);
        let r = featured_score(&s);
        assert_eq!(
            r.because,
            "Featured because 17 overtakes in window · 83 RP cover P1–P3"
        );
        assert!(r.because.contains("17"));
        assert!(r.because.contains("83"));
        assert!(!r.because.contains("25"));
        assert!(!r.because.contains("2500"));
    }

    #[test]
    fn pick_featured_prefers_higher_score() {
        let low = signals("pace_lap", "Pace Lap", 10, 0, 0, 0, 0, 0, 0, 1, 5);
        let high = signals("grand_tour", "Grand Tour", 80, 40, 20, 10, 10, 10, 10, 9, 30);
        let picked = pick_featured(&[low, high]).expect("non-empty");
        assert_eq!(picked.window_slug, "grand_tour");
        assert!(picked.score > 250);
    }

    #[test]
    fn pick_featured_tie_keeps_first() {
        let a = signals("green_flag", "Green Flag", 50, 0, 0, 0, 0, 0, 0, 4, 10);
        let b = signals("pace_lap", "Pace Lap", 50, 0, 0, 0, 0, 0, 0, 4, 10);
        let picked = pick_featured(&[a, b]).expect("non-empty");
        assert_eq!(picked.window_slug, "green_flag");
        assert_eq!(picked.score, 1250);
    }

    #[test]
    fn pick_featured_empty_is_none() {
        assert_eq!(pick_featured(&[]), None);
    }
}
