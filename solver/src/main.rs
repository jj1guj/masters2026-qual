use ac_library::MfGraph;
use proconio::input;
use rand::prelude::*;
use std::time::Instant;

const N: usize = 20;
const SA_TIME_LIMIT_MS: u128 = 800; // SA time budget in ms

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

        // Pre-compute coverage bitmasks for all (automaton, position, direction) candidates
        let mut cand_auto: Vec<usize> = Vec::new();
        let mut cand_r: Vec<usize> = Vec::new();
        let mut cand_c: Vec<usize> = Vec::new();
        let mut cand_d: Vec<usize> = Vec::new();
        let mut cand_mask: Vec<[u32; N]> = Vec::new();

        for (ai, automaton) in automata.iter().enumerate() {
            for r in 0..n {
                for c in 0..n {
                    for d in 0..4 {
                        let covered = self.simulate_automaton(automaton, r, c, d);
                        let mut mask = [0u32; N];
                        for i in 0..n {
                            for j in 0..n {
                                if covered[i][j] {
                                    mask[i] |= 1 << j;
                                }
                            }
                        }
                        cand_auto.push(ai);
                        cand_r.push(r);
                        cand_c.push(c);
                        cand_d.push(d);
                        cand_mask.push(mask);
                    }
                }
            }
        }
        let num_candidates = cand_auto.len();

        // === Approach 1: Pure vertex cover (no snakes) ===
        let no_cover = vec![vec![false; n]; n];
        let (vc_robots_pure, vc_cost_pure) = self.vertex_cover_for_uncovered(&no_cover);

        // === Approach 2: Greedy multi-snake + exact VC ===
        let mut selected: Vec<usize> = Vec::new();
        let mut combined_mask = [0u32; N];
        let mut snake_states: i64 = 0;
        let mut current_cost = vc_cost_pure;

        loop {
            let mut best_ci: Option<usize> = None;
            let mut best_total: i64 = current_cost;

            for ci in 0..num_candidates {
                // Fast new-cell count using bitmasks
                let mut new_cells = 0u32;
                for i in 0..n {
                    new_cells += (cand_mask[ci][i] & !combined_mask[i]).count_ones();
                }
                if new_cells < 3 {
                    continue;
                }

                let auto_states = automata[cand_auto[ci]].len() as i64;

                // Quick approximate filter: skip if approx can't beat best
                let mut merged_mask = combined_mask;
                for i in 0..n {
                    merged_mask[i] |= cand_mask[ci][i];
                }
                let approx_remain = self.fast_vc_estimate(&merged_mask);
                let approx_total = snake_states + auto_states + approx_remain;
                if approx_total > best_total {
                    continue;
                }

                // Convert to Vec<Vec<bool>> for exact VC
                let mut merged = vec![vec![false; n]; n];
                for i in 0..n {
                    for j in 0..n {
                        if (merged_mask[i] >> j) & 1 == 1 {
                            merged[i][j] = true;
                        }
                    }
                }
                let (_, vc_remain) = self.vertex_cover_for_uncovered(&merged);
                let total = snake_states + auto_states + vc_remain;

                if total < best_total {
                    best_total = total;
                    best_ci = Some(ci);
                }
            }

            if let Some(ci) = best_ci {
                let auto_states = automata[cand_auto[ci]].len() as i64;
                snake_states += auto_states;
                for i in 0..n {
                    combined_mask[i] |= cand_mask[ci][i];
                }
                selected.push(ci);
                current_cost = best_total;
            } else {
                break;
            }
        }

        // Compute initial approx cost for SA
        let mut current_approx: i64 = {
            let mut states: i64 = 0;
            for &ci in &selected {
                states += automata[cand_auto[ci]].len() as i64;
            }
            states + self.fast_vc_estimate(&combined_mask)
        };

        // === Simulated Annealing with fast evaluation ===
        let sa_start = Instant::now();
        let mut rng = SmallRng::seed_from_u64(42);
        let mut best_selected = selected.clone();
        let mut best_approx = current_approx;
        let max_selected = 20usize;

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

            let op = if selected.is_empty() {
                0
            } else {
                rng.random_range(0..3)
            };

            let mut new_selected = selected.clone();
            match op {
                0 => {
                    if new_selected.len() >= max_selected {
                        iter_count += 1;
                        continue;
                    }
                    let ci = rng.random_range(0..num_candidates);
                    new_selected.push(ci);
                }
                1 => {
                    let idx = rng.random_range(0..new_selected.len());
                    new_selected.remove(idx);
                }
                2 => {
                    let idx = rng.random_range(0..new_selected.len());
                    let ci = rng.random_range(0..num_candidates);
                    new_selected[idx] = ci;
                }
                _ => unreachable!(),
            }

            // Fast cost computation with bitmasks
            let mut new_mask = [0u32; N];
            let mut new_states: i64 = 0;
            for &ci in &new_selected {
                new_states += automata[cand_auto[ci]].len() as i64;
                for i in 0..n {
                    new_mask[i] |= cand_mask[ci][i];
                }
            }
            let new_vc = self.fast_vc_estimate(&new_mask);
            let new_approx = new_states + new_vc;

            let delta = new_approx - current_approx;
            let accept = if delta <= 0 {
                true
            } else {
                let prob = (-delta as f64 / temperature).exp();
                rng.random::<f64>() < prob
            };

            if accept {
                selected = new_selected;
                combined_mask = new_mask;
                current_approx = new_approx;

                if current_approx < best_approx {
                    best_approx = current_approx;
                    best_selected = selected.clone();
                }
            }

            iter_count += 1;
        }

        // Use the best found solution
        selected = best_selected;

        // Recompute combined coverage for exact VC computation at output
        combined_mask = [0u32; N];
        for &ci in &selected {
            for i in 0..n {
                combined_mask[i] |= cand_mask[ci][i];
            }
        }
        let mut combined = vec![vec![false; n]; n];
        for i in 0..n {
            for j in 0..n {
                if (combined_mask[i] >> j) & 1 == 1 {
                    combined[i][j] = true;
                }
            }
        }

        let (remain_robots, vc_cost_exact) = self.vertex_cover_for_uncovered(&combined);
        let snake_states_total: i64 = selected
            .iter()
            .map(|&ci| automata[cand_auto[ci]].len() as i64)
            .sum();
        let current_cost = snake_states_total + vc_cost_exact;

        eprintln!(
            "SA iterations: {}, approx: {}, exact: {}",
            iter_count, best_approx, current_cost
        );

        // Compare snake+VC vs pure VC and output the better one
        if current_cost < vc_cost_pure {
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
    /// Tries two strategies and returns the cheaper one:
    /// 1. "Broken" segments: break at walls AND covered cells (current)
    /// 2. "Mega" segments: break only at walls (allows redundant coverage)
    fn vertex_cover_for_uncovered(&self, pre_covered: &[Vec<bool>]) -> (Vec<String>, i64) {
        let (robots_broken, cost_broken) = self.vc_internal(pre_covered, true);
        let (robots_mega, cost_mega) = self.vc_internal(pre_covered, false);
        if cost_mega < cost_broken {
            (robots_mega, cost_mega)
        } else {
            (robots_broken, cost_broken)
        }
    }

    /// Internal VC computation.
    /// `break_at_covered=true`: segments break at walls AND pre-covered cells (fine-grained)
    /// `break_at_covered=false`: segments break only at walls (mega-segments, may cover already-covered cells)
    fn vc_internal(&self, pre_covered: &[Vec<bool>], break_at_covered: bool) -> (Vec<String>, i64) {
        let n = self.n;

        // Build row segments
        let mut row_segs: Vec<(usize, usize, usize)> = Vec::new();
        let mut row_seg_id = vec![vec![usize::MAX; n]; n];
        for i in 0..n {
            let mut j = 0;
            while j < n {
                if break_at_covered && pre_covered[i][j] {
                    j += 1;
                    continue;
                }
                let start = j;
                while j < n - 1 && !self.v[i][j] && !(break_at_covered && pre_covered[i][j + 1]) {
                    j += 1;
                }
                // For mega segments, skip if no uncovered cell exists in this segment
                if !break_at_covered {
                    let has_uncovered = (start..=j).any(|jj| !pre_covered[i][jj]);
                    if !has_uncovered {
                        j += 1;
                        continue;
                    }
                }
                let idx = row_segs.len();
                for jj in start..=j {
                    if !pre_covered[i][jj] {
                        row_seg_id[i][jj] = idx;
                    }
                }
                row_segs.push((i, start, j));
                j += 1;
            }
        }

        // Build column segments
        let mut col_segs: Vec<(usize, usize, usize)> = Vec::new();
        let mut col_seg_id = vec![vec![usize::MAX; n]; n];
        for j in 0..n {
            let mut i = 0;
            while i < n {
                if break_at_covered && pre_covered[i][j] {
                    i += 1;
                    continue;
                }
                let start = i;
                while i < n - 1 && !self.h[i][j] && !(break_at_covered && pre_covered[i + 1][j]) {
                    i += 1;
                }
                if !break_at_covered {
                    let has_uncovered = (start..=i).any(|ii| !pre_covered[ii][j]);
                    if !has_uncovered {
                        i += 1;
                        continue;
                    }
                }
                let idx = col_segs.len();
                for ii in start..=i {
                    if !pre_covered[ii][j] {
                        col_seg_id[ii][j] = idx;
                    }
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

    /// Fast VC cost estimate without max-flow. O(N^2) per call.
    /// Computes min(row_seg_cost, col_seg_cost) for both broken and mega modes.
    fn fast_vc_estimate(&self, combined_mask: &[u32; N]) -> i64 {
        let a = self.fast_vc_one_mode(combined_mask, true);
        let b = self.fast_vc_one_mode(combined_mask, false);
        std::cmp::min(a, b)
    }

    /// Estimate VC cost for one segmentation mode.
    /// break_at_covered=true: segments break at walls + covered cells
    /// break_at_covered=false: segments break only at walls (mega-segments)
    fn fast_vc_one_mode(&self, combined_mask: &[u32; N], break_at_covered: bool) -> i64 {
        let n = self.n;
        let full = (1u32 << n) - 1;

        // Row segment costs
        let mut row_cost: i64 = 0;
        for i in 0..n {
            let uncov = !combined_mask[i] & full;
            if uncov == 0 {
                continue;
            }
            let mut j = 0;
            while j < n {
                if break_at_covered && (uncov >> j) & 1 == 0 {
                    j += 1;
                    continue;
                }
                let start = j;
                loop {
                    j += 1;
                    if j >= n {
                        break;
                    }
                    if self.v[i][j - 1] {
                        break;
                    }
                    if break_at_covered && (uncov >> j) & 1 == 0 {
                        break;
                    }
                }
                // Segment [start, j-1]
                if !break_at_covered {
                    let seg_mask = if j - start >= 32 {
                        full
                    } else {
                        ((1u32 << (j - start)) - 1) << start
                    };
                    if uncov & seg_mask == 0 {
                        continue;
                    }
                }
                let seg_len = j - start;
                row_cost += if seg_len == 1 { 1 } else { 2 };
            }
        }

        // Column segment costs
        let mut col_cost: i64 = 0;
        for j in 0..n {
            // Quick check: any uncovered cell in this column?
            let mut any_uncov = false;
            for i in 0..n {
                if (combined_mask[i] >> j) & 1 == 0 {
                    any_uncov = true;
                    break;
                }
            }
            if !any_uncov {
                continue;
            }
            let mut i = 0;
            while i < n {
                let uncov_cell = (combined_mask[i] >> j) & 1 == 0;
                if break_at_covered && !uncov_cell {
                    i += 1;
                    continue;
                }
                let start = i;
                loop {
                    i += 1;
                    if i >= n {
                        break;
                    }
                    if self.h[i - 1][j] {
                        break;
                    }
                    let uc = (combined_mask[i] >> j) & 1 == 0;
                    if break_at_covered && !uc {
                        break;
                    }
                }
                // Segment [start, i-1]
                if !break_at_covered {
                    let has_uncov = (start..i).any(|ii| (combined_mask[ii] >> j) & 1 == 0);
                    if !has_uncov {
                        continue;
                    }
                }
                let seg_len = i - start;
                col_cost += if seg_len == 1 { 1 } else { 2 };
            }
        }

        std::cmp::min(row_cost, col_cost)
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
