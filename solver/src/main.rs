use ac_library::MfGraph;
use proconio::input;
use rand::prelude::*;
use std::time::Instant;

const N: usize = 20;
const SA_TIME_LIMIT_MS: u128 = 1000; // SA time budget in ms

// Direction: 0=U, 1=R, 2=D, 3=L
const DI: [i32; 4] = [-1, 0, 1, 0];
const DJ: [i32; 4] = [0, 1, 0, -1];

fn turn_right(d: usize) -> usize {
    (d + 1) % 4
}
fn turn_left(d: usize) -> usize {
    (d + 3) % 4
}

/// 6-state snake automaton definition
/// Each state: (action_no_wall, next_no_wall, action_wall, next_wall)
/// action: 0=F, 1=R, 2=L
const SNAKE_AUTOMATON: [(u8, usize, u8, usize); 6] = [
    (0, 0, 1, 1), // state 0: F→0, R→1
    (0, 2, 1, 2), // state 1: F→2, R→2
    (1, 3, 1, 3), // state 2: R→3, R→3
    (0, 3, 2, 4), // state 3: F→3, L→4
    (0, 5, 2, 5), // state 4: F→5, L→5
    (2, 0, 2, 0), // state 5: L→0, L→0
];

/// 6-state reverse snake automaton (mirror of snake: swaps R↔L)
const REVERSE_SNAKE_AUTOMATON: [(u8, usize, u8, usize); 6] = [
    (0, 0, 2, 1), // state 0: F→0, L→1
    (0, 2, 2, 2), // state 1: F→2, L→2
    (2, 3, 2, 3), // state 2: L→3, L→3
    (0, 3, 1, 4), // state 3: F→3, R→4
    (0, 5, 1, 5), // state 4: F→5, R→5
    (1, 0, 1, 0), // state 5: R→0, R→0
];

/// 2-state automaton #1: avg_cov=215, eff=107.6
/// s0: L→1, wall:R→1 | s1: F→0, wall:L→1
const AUTO_2S_A: [(u8, usize, u8, usize); 2] = [(2, 1, 1, 1), (0, 0, 2, 1)];
/// Mirror of AUTO_2S_A
const AUTO_2S_B: [(u8, usize, u8, usize); 2] = [(0, 1, 2, 0), (2, 0, 1, 0)];

/// 2-state automaton #2: avg_cov=209, eff=104.5
/// s0: R→1, wall:L→1 | s1: F→0, wall:R→1
const AUTO_2S_C: [(u8, usize, u8, usize); 2] = [(1, 1, 2, 1), (0, 0, 1, 1)];
/// Mirror of AUTO_2S_C
const AUTO_2S_D: [(u8, usize, u8, usize); 2] = [(0, 1, 1, 0), (1, 0, 2, 0)];

/// 3-state automaton #1: avg_cov=285.8, eff=95.3
const AUTO_3S_A: [(u8, usize, u8, usize); 3] = [(1, 1, 2, 2), (2, 2, 2, 1), (0, 0, 1, 2)];
/// Mirror of AUTO_3S_A
const AUTO_3S_B: [(u8, usize, u8, usize); 3] = [(2, 1, 1, 2), (1, 2, 1, 1), (0, 0, 2, 2)];

struct Solver {
    n: usize,
    a_k: i64,
    a_m: i64,
    a_w: i64,
    /// v[i][j]: wall between (i,j) and (i,j+1)
    v: Vec<Vec<bool>>,
    /// h[i][j]: wall between (i,j) and (i+1,j)
    h: Vec<Vec<bool>>,
}

impl Solver {
    fn new() -> Self {
        input! {
            n: usize,
            a_k: i64,
            a_m: i64,
            a_w: i64,
            wall_v: [String; N],
            wall_h: [String; N - 1],
        }

        let v: Vec<Vec<bool>> = wall_v
            .iter()
            .map(|s| s.chars().map(|c| c == '1').collect())
            .collect();
        let h: Vec<Vec<bool>> = wall_h
            .iter()
            .map(|s| s.chars().map(|c| c == '1').collect())
            .collect();

        Solver {
            n,
            a_k,
            a_m,
            a_w,
            v,
            h,
        }
    }

    /// Check if there's a wall in front of (r,c) facing direction d
    fn has_wall_ahead(&self, r: usize, c: usize, d: usize) -> bool {
        let n = self.n;
        match d {
            0 => r == 0 || self.h[r - 1][c], // U: wall between (r-1,c) and (r,c)
            1 => c == n - 1 || self.v[r][c], // R: wall between (r,c) and (r,c+1)
            2 => r == n - 1 || self.h[r][c], // D: wall between (r,c) and (r+1,c)
            3 => c == 0 || self.v[r][c - 1], // L: wall between (r,c-1) and (r,c)
            _ => unreachable!(),
        }
    }

    /// Simulate a given automaton and return the set of cells in the periodic cycle.
    /// automaton: slice of (action_no_wall, next_no_wall, action_wall, next_wall)
    /// Returns: covered cells as a bitmask (N*N bits for convenience, use Vec<bool>)
    fn simulate_automaton(
        &self,
        automaton: &[(u8, usize, u8, usize)],
        start_r: usize,
        start_c: usize,
        start_d: usize,
    ) -> Vec<Vec<bool>> {
        let n = self.n;
        let m = automaton.len();
        // State space: (r, c, d, s) → N * N * 4 * m
        let state_size = n * n * 4 * m;
        let encode =
            |r: usize, c: usize, d: usize, s: usize| -> usize { ((r * n + c) * 4 + d) * m + s };

        let mut visited = vec![false; state_size];
        let mut step_of = vec![0u32; state_size]; // step when first visited
        let mut trajectory: Vec<(usize, usize)> = Vec::new(); // (r, c) at each step

        let mut r = start_r;
        let mut c = start_c;
        let mut d = start_d;
        let mut s = 0usize;
        let mut step = 0u32;

        loop {
            let sid = encode(r, c, d, s);
            if visited[sid] {
                // Found cycle: periodic part is from step_of[sid] to step-1
                let cycle_start = step_of[sid] as usize;
                let mut covered = vec![vec![false; n]; n];
                for &(cr, cc) in &trajectory[cycle_start..] {
                    covered[cr][cc] = true;
                }
                return covered;
            }
            visited[sid] = true;
            step_of[sid] = step;
            trajectory.push((r, c));

            // Determine action
            let wall = self.has_wall_ahead(r, c, d);
            let (action, next_s) = if wall {
                (automaton[s].2, automaton[s].3)
            } else {
                (automaton[s].0, automaton[s].1)
            };

            // Execute action
            match action {
                0 => {
                    // Forward
                    let nr = (r as i32 + DI[d]) as usize;
                    let nc = (c as i32 + DJ[d]) as usize;
                    r = nr;
                    c = nc;
                }
                1 => {
                    // Right turn (no movement)
                    d = turn_right(d);
                }
                2 => {
                    // Left turn (no movement)
                    d = turn_left(d);
                }
                _ => unreachable!(),
            }
            s = next_s;
            step += 1;

            if step as usize > state_size {
                // Should not happen, but safety
                break;
            }
        }
        vec![vec![false; n]; n]
    }

    fn dir_char(d: usize) -> char {
        match d {
            0 => 'U',
            1 => 'R',
            2 => 'D',
            3 => 'L',
            _ => unreachable!(),
        }
    }

    fn action_char(a: u8) -> char {
        match a {
            0 => 'F',
            1 => 'R',
            2 => 'L',
            _ => unreachable!(),
        }
    }

    /// Print automaton robot definition to stdout
    fn print_automaton(automaton: &[(u8, usize, u8, usize)], r: usize, c: usize, d: usize) {
        let m = automaton.len();
        println!("{} {} {} {}", m, r, c, Self::dir_char(d));
        for &(a0, b0, a1, b1) in automaton {
            println!(
                "{} {} {} {}",
                Self::action_char(a0),
                b0,
                Self::action_char(a1),
                b1
            );
        }
    }

    fn solve(&self) {
        let n = self.n;

        // All automaton patterns to try
        let automata: Vec<&[(u8, usize, u8, usize)]> = vec![
            &AUTO_2S_A,
            &AUTO_2S_B,
            &AUTO_2S_C,
            &AUTO_2S_D,
            &AUTO_3S_A,
            &AUTO_3S_B,
            &SNAKE_AUTOMATON,
            &REVERSE_SNAKE_AUTOMATON,
        ];

        // Pre-compute coverage for all (automaton, position, direction) candidates
        let mut cand_auto: Vec<usize> = Vec::new();
        let mut cand_r: Vec<usize> = Vec::new();
        let mut cand_c: Vec<usize> = Vec::new();
        let mut cand_d: Vec<usize> = Vec::new();
        let mut cand_covered: Vec<Vec<Vec<bool>>> = Vec::new();

        for (ai, automaton) in automata.iter().enumerate() {
            for r in 0..n {
                for c in 0..n {
                    for d in 0..4 {
                        let covered = self.simulate_automaton(automaton, r, c, d);
                        cand_auto.push(ai);
                        cand_r.push(r);
                        cand_c.push(c);
                        cand_d.push(d);
                        cand_covered.push(covered);
                    }
                }
            }
        }
        let num_candidates = cand_auto.len();

        // === Approach 1: Pure vertex cover (no snakes) ===
        let no_cover = vec![vec![false; n]; n];
        let (vc_robots_pure, vc_cost_pure) = self.vertex_cover_for_uncovered(&no_cover);

        // === Approach 2: Greedy multi-snake + VC (initial solution for SA) ===
        let mut selected: Vec<usize> = Vec::new();
        let mut combined = vec![vec![false; n]; n];
        let mut snake_states: i64 = 0;
        let mut current_cost = vc_cost_pure;

        loop {
            let mut best_ci: Option<usize> = None;
            let mut best_total: i64 = current_cost;

            for ci in 0..num_candidates {
                let mut new_cells = 0usize;
                for i in 0..n {
                    for j in 0..n {
                        if cand_covered[ci][i][j] && !combined[i][j] {
                            new_cells += 1;
                        }
                    }
                }
                if new_cells < 3 {
                    continue;
                }

                let auto_states = automata[cand_auto[ci]].len();
                let mut merged = combined.clone();
                for i in 0..n {
                    for j in 0..n {
                        if cand_covered[ci][i][j] {
                            merged[i][j] = true;
                        }
                    }
                }

                let (_, vc_remain) = self.vertex_cover_for_uncovered(&merged);
                let total = snake_states + auto_states as i64 + vc_remain;

                if total < best_total {
                    best_total = total;
                    best_ci = Some(ci);
                }
            }

            if let Some(ci) = best_ci {
                let auto_states = automata[cand_auto[ci]].len() as i64;
                snake_states += auto_states;
                for i in 0..n {
                    for j in 0..n {
                        if cand_covered[ci][i][j] {
                            combined[i][j] = true;
                        }
                    }
                }
                selected.push(ci);
                current_cost = best_total;
            } else {
                break;
            }
        }

        // Helper: compute cost from a selection of candidates
        let compute_cost =
            |sel: &[usize], automata: &[&[(u8, usize, u8, usize)]]| -> (Vec<Vec<bool>>, i64) {
                let mut comb = vec![vec![false; n]; n];
                let mut states: i64 = 0;
                for &ci in sel {
                    states += automata[cand_auto[ci]].len() as i64;
                    for i in 0..n {
                        for j in 0..n {
                            if cand_covered[ci][i][j] {
                                comb[i][j] = true;
                            }
                        }
                    }
                }
                let (_, vc_cost) = self.vertex_cover_for_uncovered(&comb);
                (comb, states + vc_cost)
            };

        // === Simulated Annealing ===
        let sa_start = Instant::now();
        let mut rng = SmallRng::seed_from_u64(42);
        let mut best_selected = selected.clone();
        let mut best_cost = current_cost;
        let max_selected = 20usize; // cap on number of snakes

        let t_start: f64 = 5.0;
        let t_end: f64 = 0.1;
        let mut iter_count = 0u64;

        loop {
            let elapsed = sa_start.elapsed().as_millis();
            if elapsed >= SA_TIME_LIMIT_MS {
                break;
            }
            let progress = elapsed as f64 / SA_TIME_LIMIT_MS as f64;
            let temperature = t_start * (t_end / t_start).powf(progress);

            // Choose a neighbor operation
            let op = if selected.is_empty() {
                0 // must add
            } else {
                rng.random_range(0..3)
            };

            let mut new_selected = selected.clone();
            match op {
                0 => {
                    // Add a random candidate
                    if new_selected.len() >= max_selected {
                        iter_count += 1;
                        continue;
                    }
                    let ci = rng.random_range(0..num_candidates);
                    new_selected.push(ci);
                }
                1 => {
                    // Remove a random snake
                    let idx = rng.random_range(0..new_selected.len());
                    new_selected.remove(idx);
                }
                2 => {
                    // Replace a random snake with a random candidate
                    let idx = rng.random_range(0..new_selected.len());
                    let ci = rng.random_range(0..num_candidates);
                    new_selected[idx] = ci;
                }
                _ => unreachable!(),
            }

            let (new_combined, new_cost) = compute_cost(&new_selected, &automata);

            let delta = new_cost - current_cost;
            let accept = if delta <= 0 {
                true
            } else {
                let prob = (-delta as f64 / temperature).exp();
                rng.random::<f64>() < prob
            };

            if accept {
                selected = new_selected;
                combined = new_combined;
                current_cost = new_cost;

                if current_cost < best_cost {
                    best_cost = current_cost;
                    best_selected = selected.clone();
                }
            }

            iter_count += 1;
        }

        // Use the best found solution
        selected = best_selected;
        current_cost = best_cost;

        // Recompute combined for output
        combined = vec![vec![false; n]; n];
        for &ci in &selected {
            for i in 0..n {
                for j in 0..n {
                    if cand_covered[ci][i][j] {
                        combined[i][j] = true;
                    }
                }
            }
        }

        eprintln!("SA iterations: {}, best cost: {}", iter_count, best_cost);

        // Compare pure VC vs greedy snake+VC and output the better one
        if current_cost < vc_cost_pure {
            let (remain_robots, _) = self.vertex_cover_for_uncovered(&combined);
            let total_robots = selected.len() + remain_robots.len();
            println!("{}", total_robots);

            for &ci in &selected {
                Self::print_automaton(automata[cand_auto[ci]], cand_r[ci], cand_c[ci], cand_d[ci]);
            }

            for robot in &remain_robots {
                print!("{}", robot);
            }
        } else {
            println!("{}", vc_robots_pure.len());
            for robot in &vc_robots_pure {
                print!("{}", robot);
            }
        }

        self.output_no_walls();
    }

    /// Compute vertex cover for cells NOT already covered.
    /// `pre_covered[i][j]` = true means cell (i,j) is already covered (skip it).
    /// Returns: (robot output strings, total state cost)
    fn vertex_cover_for_uncovered(&self, pre_covered: &[Vec<bool>]) -> (Vec<String>, i64) {
        let n = self.n;

        // Build row segments restricted to uncovered cells
        let mut row_segs: Vec<(usize, usize, usize)> = Vec::new();
        let mut row_seg_id = vec![vec![usize::MAX; n]; n];
        for i in 0..n {
            let mut j = 0;
            while j < n {
                // Skip covered cells
                if pre_covered[i][j] {
                    j += 1;
                    continue;
                }
                let start = j;
                // Extend segment: stop at wall OR covered cell
                while j < n - 1 && !self.v[i][j] && !pre_covered[i][j + 1] {
                    j += 1;
                }
                let idx = row_segs.len();
                for jj in start..=j {
                    row_seg_id[i][jj] = idx;
                }
                row_segs.push((i, start, j));
                j += 1;
            }
        }

        // Build column segments restricted to uncovered cells
        let mut col_segs: Vec<(usize, usize, usize)> = Vec::new();
        let mut col_seg_id = vec![vec![usize::MAX; n]; n];
        for j in 0..n {
            let mut i = 0;
            while i < n {
                if pre_covered[i][j] {
                    i += 1;
                    continue;
                }
                let start = i;
                while i < n - 1 && !self.h[i][j] && !pre_covered[i + 1][j] {
                    i += 1;
                }
                let idx = col_segs.len();
                for ii in start..=i {
                    col_seg_id[ii][j] = idx;
                }
                col_segs.push((j, start, i));
                i += 1;
            }
        }

        let p = row_segs.len();
        let q = col_segs.len();
        if p == 0 && q == 0 {
            return (vec![], 0);
        }

        let seg_cost = |s: usize, e: usize| -> i64 { if s == e { 1 } else { 2 } };

        let source = 0;
        let sink = 1;
        let total_nodes = 2 + p + q;
        let mut graph = MfGraph::new(total_nodes);

        for i in 0..p {
            let (_, s, e) = row_segs[i];
            graph.add_edge(source, 2 + i, seg_cost(s, e));
        }
        for j in 0..q {
            let (_, s, e) = col_segs[j];
            graph.add_edge(2 + p + j, sink, seg_cost(s, e));
        }
        // Add edges only for uncovered cells
        for i in 0..n {
            for j in 0..n {
                if pre_covered[i][j] {
                    continue;
                }
                let r = row_seg_id[i][j];
                let c = col_seg_id[i][j];
                if r != usize::MAX && c != usize::MAX {
                    graph.add_edge(2 + r, 2 + p + c, i64::MAX / 2);
                }
            }
        }

        let min_cost = graph.flow(source, sink);
        let reachable = graph.min_cut(source);

        let mut selected_row = vec![false; p];
        let mut selected_col = vec![false; q];
        for i in 0..p {
            selected_row[i] = !reachable[2 + i];
        }
        for j in 0..q {
            selected_col[j] = reachable[2 + p + j];
        }

        let mut robots = Vec::new();

        for i in 0..p {
            if !selected_row[i] {
                continue;
            }
            let (row, sc, ec) = row_segs[i];
            if sc == ec {
                robots.push(format!("1 {} {} R\nR 0 R 0\n", row, sc));
            } else {
                robots.push(format!("2 {} {} R\nF 0 R 1\nR 0 R 0\n", row, sc));
            }
        }

        for j in 0..q {
            if !selected_col[j] {
                continue;
            }
            let (col, sr, er) = col_segs[j];
            if sr == er {
                robots.push(format!("1 {} {} D\nR 0 R 0\n", sr, col));
            } else {
                robots.push(format!("2 {} {} D\nF 0 R 1\nR 0 R 0\n", sr, col));
            }
        }

        (robots, min_cost)
    }

    fn output_no_walls(&self) {
        let n = self.n;
        for _ in 0..n {
            println!("{}", "0".repeat(n - 1));
        }
        for _ in 0..n - 1 {
            println!("{}", "0".repeat(n));
        }
    }
}

fn main() {
    let solver = Solver::new();
    solver.solve();
}
