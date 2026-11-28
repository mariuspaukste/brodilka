//! Lines at the pub: every pub has a little table with a board of Lines on
//! it (`O_GAMEBOARD`). Tap it and you sit down to a game: the host puts up
//! Lines 98 (web/lines98, from mariuspaukste/lines-98) over the world and
//! reports each finished game's score with `lines:<score>`.
//!
//! The pub keeps the player's best on its board. Beat it and the barkeep,
//! who has money on the regulars, stands you a purse: a fifth of the new
//! score in gold, at most 300. Only a new best pays, so it cannot be farmed.
//! The best is kept in the save.
use super::*;

/// the most the barkeep pays for a new best
pub const LINES_PURSE_MAX: i64 = 300;

impl Game {
    /// Sit down at the Lines table.
    pub(super) fn sit_to_lines(&mut self) {
        if self.pl.ghost {
            self.msg("Thy hand passes through the balls.", C_SYS);
            return;
        }
        self.cancel_activity();
        self.win = Win::Lines;
        self.msg("You sit down to a game of Lines.", C_SYS);
        self.touch_ui();
    }

    /// A game of Lines is over, with this score.
    pub fn lines_score(&mut self, score: i64) {
        if self.win != Win::Lines || score < 0 {
            return;
        }
        // nothing a 9x9 board can really give
        let score = score.min(100_000);
        let old = self.pl.lines_best;
        if score <= old {
            self.msg(
                &format!("You score {score} at Lines. Thy best stands at {old}."),
                C_SYS,
            );
            return;
        }
        self.pl.lines_best = score;
        let purse = (score / 5).min(LINES_PURSE_MAX) - (old / 5).min(LINES_PURSE_MAX);
        if purse > 0 {
            self.pl.gold += purse;
            self.msg(
                &format!(
                    "You score {score} at Lines, a new best, and it is chalked on the board. The barkeep stands you {purse} gold for it."
                ),
                C_GOOD,
            );
        } else {
            self.msg(
                &format!("You score {score} at Lines, a new best, and it is chalked on the board."),
                C_GOOD,
            );
        }
        self.touch_ui();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pub_has_a_lines_table_and_a_new_best_pays() {
        let mut g = Game::new(1997);
        g.create("Player", false, 0, "warrior", [0, 0, 0]);
        // the town's pub has its table, inside, and it can be walked up to
        let (bx, by) = g
            .map
            .buildings
            .iter()
            .find(|b| b.vendor == V_BARKEEP && g.map.in_town(b.x, b.y))
            .map(|b| (b.x, b.y))
            .unwrap();
        let (tx, ty) = (bx + 3, by + 3);
        assert_eq!(g.map.obj(tx, ty), O_GAMEBOARD);
        let (sx, sy) = g.map.start;
        assert!(g.map.find_path_as(sx, sy, tx, ty, 1, PERSON).is_some());
        // tap it from beside it: sit down to a game
        g.ents[0].x = tx as f32;
        g.ents[0].y = (ty - 1) as f32;
        g.ents[0].path.clear();
        g.snap_camera();
        g.win = Win::None;
        let (px, py) = g.scr_h(tx as f32, ty as f32, g.map.surface(tx, ty));
        g.tap(px, py - 6.0 * g.zoom);
        for _ in 0..60 {
            g.update(1.0 / 30.0);
        }
        assert_eq!(g.win, Win::Lines, "{:?}", g.journal.back());
        assert!(g.ui_json().contains("\"lines\""));
        // a first game: a new best, and a purse
        let gold = g.pl.gold;
        g.ui_cmd("lines:120");
        assert_eq!(g.pl.lines_best, 120);
        assert_eq!(g.pl.gold, gold + 24);
        // a worse one pays nothing; a better one pays the difference
        g.ui_cmd("lines:80");
        assert_eq!(g.pl.gold, gold + 24);
        g.ui_cmd("lines:200");
        assert_eq!(g.pl.gold, gold + 40);
        // kept in the save
        let s = g.save();
        let mut h = Game::new(1997);
        h.load(&s);
        assert_eq!(h.pl.lines_best, 200);
        // and only at the table
        g.ui_cmd("close");
        g.ui_cmd("lines:5000");
        assert_eq!(g.pl.lines_best, 200);
    }
}
