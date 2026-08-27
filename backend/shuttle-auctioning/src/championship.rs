//! Championship points overlay. Rank stays windowed RP; this table is POINTS.
//!
//! Grand Tour (GP display): P1..P10 = 25,18,15,12,10,8,6,4,2,1.
//! Sprint / Green Flag: P1..P3 = 8,7,6.
//! Fastest pace flag: +1. No purchasable velocity.

const GP_TABLE: [i32; 10] = [25, 18, 15, 12, 10, 8, 6, 4, 2, 1];
const SPRINT_TABLE: [i32; 3] = [8, 7, 6];

/// Grand Tour points for a finishing place. P11+ and non-positive → 0.
pub fn gp_points(place: i32) -> i32 {
    table_points(&GP_TABLE, place)
}

/// Sprint / Green Flag points. P1=8, P2=7, P3=6, else 0.
pub fn sprint_points(place: i32) -> i32 {
    table_points(&SPRINT_TABLE, place)
}

/// Fastest-pace flag: +1 when set, else 0.
pub fn fastest_pace_bonus(is_fastest: bool) -> i32 {
    if is_fastest {
        1
    } else {
        0
    }
}

fn table_points(table: &[i32], place: i32) -> i32 {
    if place < 1 {
        return 0;
    }
    table
        .get((place as usize).saturating_sub(1))
        .copied()
        .unwrap_or(0)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionKind {
    GrandTour,
    Sprint,
}

#[derive(Debug, Clone)]
pub struct SessionResult {
    pub handle: String,
    pub place: i32,
    pub kind: SessionKind,
    pub fastest_pace: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChampionshipRow {
    pub handle: String,
    pub points: i32,
    pub wins: i32,
    pub best_finish: i32,
    pub rank: i32,
}

/// Fold session results into standings.
/// Sort: points desc, wins desc, best_finish asc (1 is best), handle asc.
/// Rank is 1..n after that order (handle is the unique last key).
pub fn accumulate(results: &[SessionResult]) -> Vec<ChampionshipRow> {
    use std::collections::HashMap;

    struct Acc {
        points: i32,
        wins: i32,
        best_finish: i32,
    }

    let mut by_handle: HashMap<String, Acc> = HashMap::new();
    for r in results {
        let pts = session_points(r);
        let win = i32::from(r.place == 1);
        by_handle
            .entry(r.handle.clone())
            .and_modify(|a| {
                a.points += pts;
                a.wins += win;
                if r.place < a.best_finish {
                    a.best_finish = r.place;
                }
            })
            .or_insert(Acc {
                points: pts,
                wins: win,
                best_finish: r.place,
            });
    }

    let mut rows: Vec<ChampionshipRow> = by_handle
        .into_iter()
        .map(|(handle, a)| ChampionshipRow {
            handle,
            points: a.points,
            wins: a.wins,
            best_finish: a.best_finish,
            rank: 0,
        })
        .collect();

    rows.sort_by(|a, b| {
        b.points
            .cmp(&a.points)
            .then(b.wins.cmp(&a.wins))
            .then(a.best_finish.cmp(&b.best_finish))
            .then(a.handle.cmp(&b.handle))
    });

    for (i, row) in rows.iter_mut().enumerate() {
        row.rank = (i + 1) as i32;
    }
    rows
}

fn session_points(r: &SessionResult) -> i32 {
    let base = match r.kind {
        SessionKind::GrandTour => gp_points(r.place),
        SessionKind::Sprint => sprint_points(r.place),
    };
    base + fastest_pace_bonus(r.fastest_pace)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gp(handle: &str, place: i32, fastest: bool) -> SessionResult {
        SessionResult {
            handle: handle.into(),
            place,
            kind: SessionKind::GrandTour,
            fastest_pace: fastest,
        }
    }

    fn sprint(handle: &str, place: i32, fastest: bool) -> SessionResult {
        SessionResult {
            handle: handle.into(),
            place,
            kind: SessionKind::Sprint,
            fastest_pace: fastest,
        }
    }

    #[test]
    fn gp_points_p1_p10_p11() {
        assert_eq!(gp_points(1), 25);
        assert_eq!(gp_points(2), 18);
        assert_eq!(gp_points(3), 15);
        assert_eq!(gp_points(4), 12);
        assert_eq!(gp_points(5), 10);
        assert_eq!(gp_points(6), 8);
        assert_eq!(gp_points(7), 6);
        assert_eq!(gp_points(8), 4);
        assert_eq!(gp_points(9), 2);
        assert_eq!(gp_points(10), 1);
        assert_eq!(gp_points(11), 0);
        assert_eq!(gp_points(0), 0);
        assert_eq!(gp_points(-1), 0);
    }

    #[test]
    fn sprint_points_p3_p4() {
        assert_eq!(sprint_points(1), 8);
        assert_eq!(sprint_points(2), 7);
        assert_eq!(sprint_points(3), 6);
        assert_eq!(sprint_points(4), 0);
        assert_eq!(sprint_points(0), 0);
    }

    #[test]
    fn fastest_pace_bonus_is_one_or_zero() {
        assert_eq!(fastest_pace_bonus(true), 1);
        assert_eq!(fastest_pace_bonus(false), 0);
    }

    #[test]
    fn two_gps_p1_then_p2_sum_to_43() {
        let rows = accumulate(&[gp("alice", 1, false), gp("alice", 2, false)]);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].handle, "alice");
        assert_eq!(rows[0].points, 25 + 18);
        assert_eq!(rows[0].wins, 1);
        assert_eq!(rows[0].best_finish, 1);
        assert_eq!(rows[0].rank, 1);
    }

    #[test]
    fn fastest_pace_adds_one() {
        let without = accumulate(&[gp("solo", 10, false)]);
        let with = accumulate(&[gp("solo", 10, true)]);
        assert_eq!(without[0].points, 1);
        assert_eq!(with[0].points, 2);
        assert_eq!(with[0].points, without[0].points + 1);
    }

    #[test]
    fn sort_points_wins_best_finish_handle() {
        // points desc, then wins desc, then best_finish asc, then handle asc.
        let rows = accumulate(&[
            gp("zeta", 1, false),  // 25, 1 win, best 1
            gp("nowin", 6, false), // 8, 0 wins, best 6
            sprint("haswin", 1, false), // 8, 1 win, best 1
            gp("close", 3, false), // 15
            gp("close", 10, false), // +1 → 16, 0 wins, best 3
            gp("spread", 6, false),
            gp("spread", 6, false), // 16, 0 wins, best 6
            gp("mike", 2, false),  // 18, 0 wins, best 2
            gp("alex", 2, false),  // 18, 0 wins, best 2
        ]);

        let order: Vec<&str> = rows.iter().map(|r| r.handle.as_str()).collect();
        assert_eq!(
            order,
            vec!["zeta", "alex", "mike", "close", "spread", "haswin", "nowin"]
        );
        for (i, row) in rows.iter().enumerate() {
            assert_eq!(row.rank, (i + 1) as i32);
        }
        assert_eq!(rows[0].points, 25);
        assert_eq!(rows[1].points, 18);
        assert_eq!(rows[1].handle, "alex");
        assert_eq!(rows[2].handle, "mike");
        assert_eq!(rows[3].best_finish, 3);
        assert_eq!(rows[4].best_finish, 6);
        assert_eq!(rows[5].wins, 1);
        assert_eq!(rows[6].wins, 0);
        assert_eq!(rows[5].points, 8);
        assert_eq!(rows[6].points, 8);
    }

    #[test]
    fn empty_results_empty_standings() {
        assert!(accumulate(&[]).is_empty());
    }

    #[test]
    fn sprint_p4_scores_only_fastest_bonus() {
        let rows = accumulate(&[sprint("out", 4, true)]);
        assert_eq!(rows[0].points, 1);
        assert_eq!(rows[0].wins, 0);
        assert_eq!(rows[0].best_finish, 4);
    }
}
