//! Bamboo groves in the hills of the tropics, and the giant pandas that
//! live in them (`game/panda.rs` has their ways, `bake/panda.rs` their
//! figure).
//!
//! The groves are made after the rest of the land, from their own stream of
//! chance, on high open grass in the tropics away from the guarded cities:
//! the three such places with the most of it, far apart. A grove is a ragged
//! patch of bamboo stands in the grass, crowding out the trees, which anyone
//! may walk through. Each has a pair of pandas.

use super::*;

/// how many groves, how far apart their middles at least, and their radius
const GROVES: usize = 3;
const APART: i32 = 40;
const R: i32 = 7;

impl Map {
    pub(super) fn bamboo_groves(&mut self, seed: u32) {
        // open, high grass of the tropics that the wild may have
        let open = |m: &Map, x: i32, y: i32| {
            let i = idx(x, y);
            m.tropic(x, y)
                && m.t[i] == T_GRASS
                && m.lv[i] >= 2
                && m.reach[i]
                && !m.road[i]
                && m.bld[i] == 0
                && !m.guarded(x, y)
                && matches!(m.o[i], O_NONE | O_TREE | O_BUSH | O_FLOWERS)
        };
        let mut best: Vec<(i32, i32, i32)> = vec![];
        for cy in (NORTH_H..LAND_H - R).step_by(3) {
            for cx in (R..W - R).step_by(3) {
                if !open(self, cx, cy) {
                    continue;
                }
                let mut n = 0;
                for y in cy - R..=cy + R {
                    for x in cx - R..=cx + R {
                        if inb(x, y) && open(self, x, y) {
                            n += 1;
                        }
                    }
                }
                best.push((n, cx, cy));
            }
        }
        best.sort_by(|a, b| b.0.cmp(&a.0).then((a.1, a.2).cmp(&(b.1, b.2))));
        let mut chosen: Vec<(i32, i32)> = vec![];
        for (_, x, y) in best {
            if chosen.len() == GROVES {
                break;
            }
            if chosen
                .iter()
                .all(|&(cx, cy)| (cx - x).abs().max((cy - y).abs()) >= APART)
            {
                chosen.push((x, y));
            }
        }
        for &(cx, cy) in &chosen {
            for y in cy - R..=cy + R {
                for x in cx - R..=cx + R {
                    if !inb(x, y) || !open(self, x, y) {
                        continue;
                    }
                    let d = (((x - cx).pow(2) + (y - cy).pow(2)) as f32).sqrt();
                    let edge =
                        R as f32 * (0.7 + 0.4 * fbm(x as f32 / 4.0, y as f32 / 4.0, seed ^ 0xBA));
                    // thick in the middle, thinning to the edge
                    let k = (1.0 - d / edge).clamp(0.0, 1.0);
                    if d >= edge {
                        continue;
                    }
                    // the bamboo crowds out the trees
                    self.o[idx(x, y)] = if hash(x, y, seed ^ 0xB00) < 0.3 + 0.5 * k {
                        O_BAMBOO
                    } else {
                        O_NONE
                    };
                }
            }
            self.spawns.push(("panda", cx, cy, R - 2, 2));
        }
        self.groves = chosen;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tropic_hills_have_bamboo_groves_with_pandas() {
        for seed in [1, 42, 1997] {
            let m = generate(seed);
            assert_eq!(m.groves.len(), GROVES, "seed {seed}");
            for &(x, y) in &m.groves {
                assert!(m.tropic(x, y) && !m.guarded(x, y));
                let n = (y - R..=y + R)
                    .flat_map(|yy| (x - R..=x + R).map(move |xx| (xx, yy)))
                    .filter(|&(xx, yy)| m.obj(xx, yy) == O_BAMBOO)
                    .count();
                assert!(n >= 20, "seed {seed}: only {n} stands at {x},{y}");
                assert!(m
                    .spawns
                    .iter()
                    .any(|s| s.0 == "panda" && (s.1, s.2) == (x, y)));
                // bamboo is walked through
                let (sx, sy) = m.start;
                let mut m = generate(seed);
                assert!(m.find_path(sx, sy, x, y, 1).is_some(), "seed {seed}");
            }
        }
    }
}
