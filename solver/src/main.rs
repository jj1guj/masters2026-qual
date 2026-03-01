use proconio::input;
use rand::prelude::*;
use rand::rngs::SmallRng;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::time::Instant;

const N: usize = 20;
const MAX_STATES: usize = 4 * N * N; // 1600
const TIME_LIMIT_MS: u128 = 1800;

// Directions: 0=Up, 1=Right, 2=Down, 3=Left
const DI: [i32; 4] = [-1, 0, 1, 0];
const DJ: [i32; 4] = [0, 1, 0, -1];

/// Automaton state: transitions for wall=false and wall=true
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct AutoState {
    act0: char,   // action when no wall
    next0: usize, // next state when no wall
    act1: char,   // action when wall
    next1: usize, // next state when wall
}

/// Full solution
#[derive(Debug, Clone)]
struct Solution {
    states: Vec<AutoState>,
    start_i: usize,
    start_j: usize,
    start_dir: usize,
}

struct Solver {
    _a_k: i64,
    _a_m: i64,
    _a_w: i64,
    wall_v: Vec<Vec<u8>>, // wall_v[i][j] = wall between (i,j) and (i,j+1)
    wall_h: Vec<Vec<u8>>, // wall_h[i][j] = wall between (i,j) and (i+1,j)
}

impl Solver {
    fn new() -> Self {
        input! {
            _n: usize,
            a_k: i64,
            a_m: i64,
            a_w: i64,
            wall_v_str: [String; N],
            wall_h_str: [String; N - 1],
        }
        let wall_v: Vec<Vec<u8>> = wall_v_str
            .iter()
            .map(|s| s.bytes().map(|b| b - b'0').collect())
            .collect();
        let wall_h: Vec<Vec<u8>> = wall_h_str
            .iter()
            .map(|s| s.bytes().map(|b| b - b'0').collect())
            .collect();
        Solver {
            _a_k: a_k,
            _a_m: a_m,
            _a_w: a_w,
            wall_v,
            wall_h,
        }
    }

    fn can_move(&self, i: usize, j: usize, d: usize) -> bool {
        match d {
            0 => i > 0 && self.wall_h[i - 1][j] == 0,
            1 => j + 1 < N && self.wall_v[i][j] == 0,
            2 => i + 1 < N && self.wall_h[i][j] == 0,
            3 => j > 0 && self.wall_v[i][j - 1] == 0,
            _ => unreachable!(),
        }
    }

    fn direction_between(i1: usize, j1: usize, i2: usize, j2: usize) -> usize {
        if i2 < i1 {
            0
        } else if j2 > j1 {
            1
        } else if i2 > i1 {
            2
        } else {
            3
        }
    }

    fn turns_needed(from: usize, to: usize) -> Vec<char> {
        match (to + 4 - from) % 4 {
            0 => vec![],
            1 => vec!['R'],
            2 => vec!['R', 'R'],
            3 => vec!['L'],
            _ => unreachable!(),
        }
    }

    fn apply_turn(dir: usize, turn: char) -> usize {
        match turn {
            'R' => (dir + 1) % 4,
            'L' => (dir + 3) % 4,
            _ => dir,
        }
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

    // ---- DFS Euler Tour ----

    fn dfs_euler_tour_from(
        &self,
        si: usize,
        sj: usize,
        rng: &mut SmallRng,
        randomize: bool,
    ) -> Vec<(usize, usize)> {
        let mut visited = vec![vec![false; N]; N];
        let mut tour = Vec::with_capacity(800);
        self.dfs_impl(si, sj, &mut visited, &mut tour, rng, randomize);
        tour
    }

    fn dfs_impl(
        &self,
        i: usize,
        j: usize,
        visited: &mut Vec<Vec<bool>>,
        tour: &mut Vec<(usize, usize)>,
        rng: &mut SmallRng,
        randomize: bool,
    ) {
        visited[i][j] = true;
        tour.push((i, j));
        let dirs: [usize; 4] = if randomize {
            let mut d = [0, 1, 2, 3];
            d.shuffle(rng);
            d
        } else if i % 2 == 0 {
            [1, 2, 0, 3]
        } else {
            [3, 2, 0, 1]
        };
        for &d in &dirs {
            if !self.can_move(i, j, d) {
                continue;
            }
            let ni = ((i as i32) + DI[d]) as usize;
            let nj = ((j as i32) + DJ[d]) as usize;
            if !visited[ni][nj] {
                self.dfs_impl(ni, nj, visited, tour, rng, randomize);
                tour.push((i, j));
            }
        }
    }

    // ---- Tour -> actions -> compressed automaton ----

    fn tour_to_actions(&self, tour: &[(usize, usize)], init_dir: usize) -> Vec<(char, bool)> {
        let mut actions = Vec::with_capacity(1200);
        let mut cur_dir = init_dir;
        for step in 0..tour.len() - 1 {
            let (ci, cj) = tour[step];
            let (ni, nj) = tour[step + 1];
            let target_dir = Self::direction_between(ci, cj, ni, nj);
            for &t in &Self::turns_needed(cur_dir, target_dir) {
                let front_wall = !self.can_move(ci, cj, cur_dir);
                actions.push((t, front_wall));
                cur_dir = Self::apply_turn(cur_dir, t);
            }
            actions.push(('F', false));
        }
        // Close loop
        let (ci, cj) = *tour.last().unwrap();
        for &t in &Self::turns_needed(cur_dir, init_dir) {
            let front_wall = !self.can_move(ci, cj, cur_dir);
            actions.push((t, front_wall));
            cur_dir = Self::apply_turn(cur_dir, t);
        }
        actions
    }

    /// Build compressed automaton: F-runs ending at wall -> single RunF state
    fn build_automaton(actions: &[(char, bool)]) -> Vec<AutoState> {
        let m = actions.len();
        let mut seg_act0: Vec<char> = Vec::new();
        let mut seg_act1: Vec<char> = Vec::new();
        let mut seg_self_loop: Vec<bool> = Vec::new();

        let mut i = 0;
        while i < m {
            if actions[i].0 == 'F' {
                let f_start = i;
                while i < m && actions[i].0 == 'F' {
                    i += 1;
                }
                if i < m && actions[i].0 != 'F' && actions[i].1 {
                    seg_act0.push('F');
                    seg_act1.push(actions[i].0);
                    seg_self_loop.push(true);
                    i += 1;
                } else {
                    for _ in f_start..i {
                        seg_act0.push('F');
                        seg_act1.push('R');
                        seg_self_loop.push(false);
                    }
                }
            } else {
                let c = actions[i].0;
                seg_act0.push(c);
                seg_act1.push(c);
                seg_self_loop.push(false);
                i += 1;
            }
        }

        let sm = seg_act0.len();
        let mut states = Vec::with_capacity(sm);
        for idx in 0..sm {
            let next = (idx + 1) % sm;
            if seg_self_loop[idx] {
                states.push(AutoState {
                    act0: 'F',
                    next0: idx,
                    act1: seg_act1[idx],
                    next1: next,
                });
            } else {
                states.push(AutoState {
                    act0: seg_act0[idx],
                    next0: next,
                    act1: seg_act1[idx],
                    next1: next,
                });
            }
        }
        states
    }

    // ---- DFA minimization (Mealy machine partition refinement) ----

    fn minimize(states: &[AutoState], initial: usize) -> (Vec<AutoState>, usize) {
        let m = states.len();
        if m <= 1 {
            return (states.to_vec(), 0);
        }

        // Reachability
        let mut reachable = vec![false; m];
        let mut stack = vec![initial];
        reachable[initial] = true;
        while let Some(s) = stack.pop() {
            for &nxt in &[states[s].next0, states[s].next1] {
                if nxt < m && !reachable[nxt] {
                    reachable[nxt] = true;
                    stack.push(nxt);
                }
            }
        }

        let ids: Vec<usize> = (0..m).filter(|&i| reachable[i]).collect();
        let rm = ids.len();
        let mut old2new = vec![0usize; m];
        for (ni, &oi) in ids.iter().enumerate() {
            old2new[oi] = ni;
        }

        let cstates: Vec<AutoState> = ids
            .iter()
            .map(|&oi| AutoState {
                act0: states[oi].act0,
                next0: old2new[states[oi].next0],
                act1: states[oi].act1,
                next1: old2new[states[oi].next1],
            })
            .collect();
        let cinit = old2new[initial];

        // Partition refinement
        let mut group = vec![0usize; rm];
        let mut sig_map: HashMap<(char, char), usize> = HashMap::new();
        let mut ng = 0usize;
        for i in 0..rm {
            let sig = (cstates[i].act0, cstates[i].act1);
            let g = *sig_map.entry(sig).or_insert_with(|| {
                let v = ng;
                ng += 1;
                v
            });
            group[i] = g;
        }

        loop {
            let mut new_map: HashMap<(usize, usize, usize), usize> = HashMap::new();
            let mut new_group = vec![0usize; rm];
            let mut new_ng = 0usize;
            for i in 0..rm {
                let sig = (group[i], group[cstates[i].next0], group[cstates[i].next1]);
                let g = *new_map.entry(sig).or_insert_with(|| {
                    let v = new_ng;
                    new_ng += 1;
                    v
                });
                new_group[i] = g;
            }
            if new_ng == ng {
                break;
            }
            group = new_group;
            ng = new_ng;
        }

        // Build minimized automaton
        let mut rep = vec![None; ng];
        for i in 0..rm {
            if rep[group[i]].is_none() {
                rep[group[i]] = Some(i);
            }
        }
        let min_states: Vec<AutoState> = (0..ng)
            .map(|g| {
                let r = rep[g].unwrap();
                AutoState {
                    act0: cstates[r].act0,
                    next0: group[cstates[r].next0],
                    act1: cstates[r].act1,
                    next1: group[cstates[r].next1],
                }
            })
            .collect();

        (min_states, group[cinit])
    }

    /// Remap so initial state becomes state 0
    fn remap_to_zero(states: &[AutoState], initial: usize) -> Vec<AutoState> {
        if initial == 0 {
            return states.to_vec();
        }
        let m = states.len();
        let mut old2new = vec![0usize; m];
        let mut visited = vec![false; m];
        let mut queue = std::collections::VecDeque::new();
        let mut order = Vec::with_capacity(m);
        queue.push_back(initial);
        visited[initial] = true;
        while let Some(s) = queue.pop_front() {
            order.push(s);
            for &nxt in &[states[s].next0, states[s].next1] {
                if nxt < m && !visited[nxt] {
                    visited[nxt] = true;
                    queue.push_back(nxt);
                }
            }
        }
        for i in 0..m {
            if !visited[i] {
                order.push(i);
            }
        }
        for (ni, &oi) in order.iter().enumerate() {
            old2new[oi] = ni;
        }
        let mut result = vec![
            AutoState {
                act0: 'R',
                next0: 0,
                act1: 'R',
                next1: 0
            };
            m
        ];
        for (oi, st) in states.iter().enumerate() {
            result[old2new[oi]] = AutoState {
                act0: st.act0,
                next0: old2new[st.next0],
                act1: st.act1,
                next1: old2new[st.next1],
            };
        }
        result
    }

    // ---- BFS helpers ----

    fn bfs_path(&self, si: usize, sj: usize, ti: usize, tj: usize) -> Vec<(usize, usize)> {
        if si == ti && sj == tj {
            return vec![(si, sj)];
        }
        let mut prev = vec![vec![(usize::MAX, usize::MAX); N]; N];
        let mut queue = VecDeque::new();
        prev[si][sj] = (si, sj);
        queue.push_back((si, sj));
        while let Some((i, j)) = queue.pop_front() {
            for d in 0..4 {
                if !self.can_move(i, j, d) {
                    continue;
                }
                let ni = ((i as i32) + DI[d]) as usize;
                let nj = ((j as i32) + DJ[d]) as usize;
                if prev[ni][nj].0 == usize::MAX {
                    prev[ni][nj] = (i, j);
                    if ni == ti && nj == tj {
                        let mut path = vec![];
                        let (mut pi, mut pj) = (ti, tj);
                        while (pi, pj) != (si, sj) {
                            path.push((pi, pj));
                            let (qi, qj) = prev[pi][pj];
                            pi = qi;
                            pj = qj;
                        }
                        path.push((si, sj));
                        path.reverse();
                        return path;
                    }
                    queue.push_back((ni, nj));
                }
            }
        }
        vec![]
    }

    fn bfs_nearest_uncovered(
        &self,
        si: usize,
        sj: usize,
        visited: &[[bool; N]; N],
    ) -> Option<(usize, usize)> {
        let mut seen = [[false; N]; N];
        let mut queue = VecDeque::new();
        seen[si][sj] = true;
        queue.push_back((si, sj));
        while let Some((i, j)) = queue.pop_front() {
            if !visited[i][j] {
                return Some((i, j));
            }
            for d in 0..4 {
                if !self.can_move(i, j, d) {
                    continue;
                }
                let ni = ((i as i32) + DI[d]) as usize;
                let nj = ((j as i32) + DJ[d]) as usize;
                if !seen[ni][nj] {
                    seen[ni][nj] = true;
                    queue.push_back((ni, nj));
                }
            }
        }
        None
    }

    // ---- Snake sweep tour ----

    /// Snake sweep: zigzag rows/cols with BFS navigation when stuck
    fn snake_sweep_tour(
        &self,
        si: usize,
        sj: usize,
        sweep_dir: usize,
        step_dir: usize,
    ) -> Vec<(usize, usize)> {
        let mut visited = [[false; N]; N];
        let mut tour = Vec::with_capacity(800);
        let mut ci = si;
        let mut cj = sj;
        let mut cur_sweep = sweep_dir;
        let mut total_visited = 0usize;

        visited[ci][cj] = true;
        total_visited += 1;
        tour.push((ci, cj));

        loop {
            // Sweep in current direction until wall or visited cell
            while self.can_move(ci, cj, cur_sweep) {
                let ni = ((ci as i32) + DI[cur_sweep]) as usize;
                let nj = ((cj as i32) + DJ[cur_sweep]) as usize;
                if visited[ni][nj] {
                    break;
                }
                ci = ni;
                cj = nj;
                visited[ci][cj] = true;
                total_visited += 1;
                tour.push((ci, cj));
            }

            if total_visited == N * N {
                break;
            }

            // Try step in secondary direction, then opposite
            let mut stepped = false;
            for &sd in &[step_dir, (step_dir + 2) % 4] {
                if self.can_move(ci, cj, sd) {
                    let ni = ((ci as i32) + DI[sd]) as usize;
                    let nj = ((cj as i32) + DJ[sd]) as usize;
                    if !visited[ni][nj] {
                        ci = ni;
                        cj = nj;
                        visited[ci][cj] = true;
                        total_visited += 1;
                        tour.push((ci, cj));
                        cur_sweep = (cur_sweep + 2) % 4; // reverse sweep
                        stepped = true;
                        break;
                    }
                }
            }
            if stepped {
                continue;
            }

            // Stuck: BFS to nearest uncovered cell
            if let Some((ti, tj)) = self.bfs_nearest_uncovered(ci, cj, &visited) {
                let path = self.bfs_path(ci, cj, ti, tj);
                for &(ni, nj) in &path[1..] {
                    tour.push((ni, nj));
                    if !visited[ni][nj] {
                        visited[ni][nj] = true;
                        total_visited += 1;
                    }
                    ci = ni;
                    cj = nj;
                }
            } else {
                break;
            }
        }

        // Return to start
        if (ci, cj) != (si, sj) {
            let path = self.bfs_path(ci, cj, si, sj);
            for &(ni, nj) in &path[1..] {
                tour.push((ni, nj));
            }
        }

        tour
    }

    // ---- Snake automata (8 states) ----

    fn snake_candidates() -> Vec<(Vec<AutoState>, usize, usize, usize)> {
        // Automaton A: R-then-L pattern
        let auto_a = vec![
            AutoState {
                act0: 'F',
                next0: 0,
                act1: 'R',
                next1: 1,
            }, // RunF, wall→R
            AutoState {
                act0: 'F',
                next0: 2,
                act1: 'R',
                next1: 1,
            }, // step 1, wall→dummy
            AutoState {
                act0: 'R',
                next0: 3,
                act1: 'R',
                next1: 3,
            }, // turn R
            AutoState {
                act0: 'F',
                next0: 3,
                act1: 'L',
                next1: 4,
            }, // RunF, wall→L
            AutoState {
                act0: 'F',
                next0: 5,
                act1: 'L',
                next1: 6,
            }, // step 1, wall→return
            AutoState {
                act0: 'L',
                next0: 0,
                act1: 'L',
                next1: 0,
            }, // turn L
            AutoState {
                act0: 'L',
                next0: 7,
                act1: 'L',
                next1: 7,
            }, // turn (return)
            AutoState {
                act0: 'F',
                next0: 7,
                act1: 'R',
                next1: 0,
            }, // RunF return
        ];
        // Automaton B: L-then-R pattern (mirror)
        let auto_b = vec![
            AutoState {
                act0: 'F',
                next0: 0,
                act1: 'L',
                next1: 1,
            },
            AutoState {
                act0: 'F',
                next0: 2,
                act1: 'L',
                next1: 1,
            },
            AutoState {
                act0: 'L',
                next0: 3,
                act1: 'L',
                next1: 3,
            },
            AutoState {
                act0: 'F',
                next0: 3,
                act1: 'R',
                next1: 4,
            },
            AutoState {
                act0: 'F',
                next0: 5,
                act1: 'R',
                next1: 6,
            },
            AutoState {
                act0: 'R',
                next0: 0,
                act1: 'R',
                next1: 0,
            },
            AutoState {
                act0: 'R',
                next0: 7,
                act1: 'R',
                next1: 7,
            },
            AutoState {
                act0: 'F',
                next0: 7,
                act1: 'L',
                next1: 0,
            },
        ];

        vec![
            (auto_a.clone(), 0, 0, 1),     // Row snake from (0,0) facing R
            (auto_b.clone(), 0, N - 1, 3), // Row snake from (0,19) facing L
            (auto_b.clone(), 0, 0, 2),     // Col snake from (0,0) facing D
            (auto_a.clone(), N - 1, 0, 0), // Col snake from (19,0) facing U
        ]
    }

    /// Simulate automaton on the actual grid and check if all N*N cells
    /// are visited in the periodic (cycle) part of the trajectory.
    fn simulate_and_check(&self, states: &[AutoState], si: usize, sj: usize, sd: usize) -> bool {
        let m = states.len();
        let total_configs = m * N * N * 4;
        let config_id =
            |i: usize, j: usize, d: usize, s: usize| -> usize { ((i * N + j) * 4 + d) * m + s };

        let mut first_seen = vec![usize::MAX; total_configs];
        let mut positions: Vec<(usize, usize)> = Vec::with_capacity(total_configs + 1);
        let (mut ci, mut cj, mut cd, mut cs) = (si, sj, sd, 0usize);

        for step in 0..=total_configs {
            let cid = config_id(ci, cj, cd, cs);
            if cid >= total_configs {
                return false;
            }
            if first_seen[cid] != usize::MAX {
                let cycle_start = first_seen[cid];
                let mut visited = [[false; N]; N];
                for k in cycle_start..step {
                    let (pi, pj) = positions[k];
                    visited[pi][pj] = true;
                }
                return (0..N).all(|i| (0..N).all(|j| visited[i][j]));
            }
            first_seen[cid] = step;
            positions.push((ci, cj));

            let front_wall = !self.can_move(ci, cj, cd);
            let (action, ns) = if front_wall {
                (states[cs].act1, states[cs].next1)
            } else {
                (states[cs].act0, states[cs].next0)
            };
            match action {
                'F' => {
                    ci = ((ci as i32) + DI[cd]) as usize;
                    cj = ((cj as i32) + DJ[cd]) as usize;
                }
                'R' => cd = (cd + 1) % 4,
                'L' => cd = (cd + 3) % 4,
                _ => {}
            }
            cs = ns;
        }
        false
    }

    // ---- Build solution from tour ----

    fn build_solution(&self, tour: &[(usize, usize)], init_dir: usize) -> Option<Solution> {
        let actions = self.tour_to_actions(tour, init_dir);
        let automaton = Self::build_automaton(&actions);
        let (min_auto, min_init) = Self::minimize(&automaton, 0);
        if min_auto.len() > MAX_STATES {
            return None;
        }
        let remapped = Self::remap_to_zero(&min_auto, min_init);
        Some(Solution {
            states: remapped,
            start_i: tour[0].0,
            start_j: tour[0].1,
            start_dir: init_dir,
        })
    }

    // ---- Main solve ----

    fn solve(&mut self) {
        let t0 = Instant::now();
        let mut rng = SmallRng::seed_from_u64(42);
        let mut best: Option<Solution> = None;
        let mut best_cost = usize::MAX;
        let mut best_tour: Option<Vec<(usize, usize)>> = None;
        let mut iters = 0u64;

        // Try snake automata (8 states each, massive improvement if they work)
        for (states, si, sj, sd) in Self::snake_candidates() {
            if self.simulate_and_check(&states, si, sj, sd) {
                let cost = states.len();
                eprintln!("Snake OK! states={} start=({},{}) dir={}", cost, si, sj, sd);
                if cost < best_cost {
                    best_cost = cost;
                    best = Some(Solution {
                        states,
                        start_i: si,
                        start_j: sj,
                        start_dir: sd,
                    });
                }
            }
        }

        // Try snake sweep variants (4 corners × 2 orientations = 8)
        let sweep_variants: [(usize, usize, usize, usize); 8] = [
            (0, 0, 1, 2),         // (0,0) sweep R, step D
            (0, N - 1, 3, 2),     // (0,19) sweep L, step D
            (N - 1, 0, 1, 0),     // (19,0) sweep R, step U
            (N - 1, N - 1, 3, 0), // (19,19) sweep L, step U
            (0, 0, 2, 1),         // (0,0) sweep D, step R
            (0, N - 1, 2, 3),     // (0,19) sweep D, step L
            (N - 1, 0, 0, 1),     // (19,0) sweep U, step R
            (N - 1, N - 1, 0, 3), // (19,19) sweep U, step L
        ];
        for &(si, sj, sweep_dir, step_dir) in &sweep_variants {
            let tour = self.snake_sweep_tour(si, sj, sweep_dir, step_dir);
            if tour.len() < 2 {
                continue;
            }
            for init_dir in 0..4 {
                if let Some(sol) = self.build_solution(&tour, init_dir) {
                    if sol.states.len() < best_cost {
                        best_cost = sol.states.len();
                        best = Some(sol);
                        best_tour = Some(tour.clone());
                    }
                }
            }
        }

        // Deterministic DFS from (0,0)
        {
            let tour = self.dfs_euler_tour_from(0, 0, &mut rng, false);
            let d = Self::direction_between(tour[0].0, tour[0].1, tour[1].0, tour[1].1);
            if let Some(sol) = self.build_solution(&tour, d) {
                if sol.states.len() < best_cost {
                    best_cost = sol.states.len();
                    best = Some(sol);
                    best_tour = Some(tour);
                }
            }
            iters += 1;
        }

        // Phase 1: Randomized search (first 1.0s)
        let phase1_limit = 1000u128;
        let sweep_dirs: [(usize, usize); 4] = [(1, 2), (3, 2), (2, 1), (0, 1)];
        while t0.elapsed().as_millis() < phase1_limit {
            let si = rng.random_range(0..N);
            let sj = rng.random_range(0..N);

            // Random sweep
            let &(sw, st) = sweep_dirs.choose(&mut rng).unwrap();
            let tour_sw = self.snake_sweep_tour(si, sj, sw, st);
            if tour_sw.len() >= 2 {
                for init_dir in 0..4 {
                    if let Some(sol) = self.build_solution(&tour_sw, init_dir) {
                        if sol.states.len() < best_cost {
                            best_cost = sol.states.len();
                            best = Some(sol);
                            best_tour = Some(tour_sw.clone());
                        }
                    }
                }
            }

            // Random DFS
            let tour = self.dfs_euler_tour_from(si, sj, &mut rng, true);
            if tour.len() >= 2 {
                for init_dir in 0..4 {
                    if let Some(sol) = self.build_solution(&tour, init_dir) {
                        if sol.states.len() < best_cost {
                            best_cost = sol.states.len();
                            best = Some(sol);
                            best_tour = Some(tour.clone());
                        }
                    }
                }
            }
            iters += 1;
        }

        eprintln!("Phase1: iters={}, best_states={}", iters, best_cost);

        // Phase 2: Hill climbing - shorten the best tour (remaining time)
        if let Some(ref base_tour) = best_tour {
            let mut cur_tour = base_tour.clone();
            let mut hc_iters = 0u64;
            let mut hc_improved = 0u64;

            while t0.elapsed().as_millis() < TIME_LIMIT_MS {
                let n = cur_tour.len();
                if n < 5 {
                    break;
                }

                // Pick random segment [i..j]
                let i = rng.random_range(0..n - 3);
                let j_max = (n - 1).min(i + 60);
                if i + 3 > j_max {
                    hc_iters += 1;
                    continue;
                }
                let j = rng.random_range((i + 3)..=j_max);

                // BFS shortest path from cur_tour[i] to cur_tour[j]
                let (si, sj) = cur_tour[i];
                let (ti, tj) = cur_tour[j];
                let path = self.bfs_path(si, sj, ti, tj);
                if path.is_empty() || path.len() >= j - i + 1 {
                    hc_iters += 1;
                    continue;
                }

                // Construct shortened tour
                let mut new_tour = Vec::with_capacity(n);
                new_tour.extend_from_slice(&cur_tour[..i]);
                new_tour.extend_from_slice(&path);
                new_tour.extend_from_slice(&cur_tour[j + 1..]);

                // Check all N*N cells still covered
                let mut covered = [[false; N]; N];
                for &(r, c) in &new_tour {
                    covered[r][c] = true;
                }
                if !(0..N).all(|r| (0..N).all(|c| covered[r][c])) {
                    hc_iters += 1;
                    continue;
                }

                // Evaluate: try all 4 init directions
                let mut found_better = false;
                for d in 0..4 {
                    if let Some(sol) = self.build_solution(&new_tour, d) {
                        if sol.states.len() < best_cost {
                            best_cost = sol.states.len();
                            best = Some(sol);
                            found_better = true;
                        }
                    }
                }

                // Accept shorter tour (even if states didn't improve, opens future shortcuts)
                if found_better || new_tour.len() < cur_tour.len() {
                    cur_tour = new_tour;
                    hc_improved += 1;
                }

                hc_iters += 1;
            }
            eprintln!(
                "HC: iters={}, improved={}, tour_len={}, best_states={}",
                hc_iters,
                hc_improved,
                cur_tour.len(),
                best_cost
            );
        }

        match best {
            Some(sol) => self.output(&sol),
            None => self.naive(),
        }
    }

    fn output(&self, sol: &Solution) {
        let m = sol.states.len();
        println!("1");
        println!(
            "{} {} {} {}",
            m,
            sol.start_i,
            sol.start_j,
            Self::dir_char(sol.start_dir)
        );
        for st in &sol.states {
            println!("{} {} {} {}", st.act0, st.next0, st.act1, st.next1);
        }
        for _ in 0..N {
            println!("{}", "0".repeat(N - 1));
        }
        for _ in 0..N - 1 {
            println!("{}", "0".repeat(N));
        }
    }

    fn naive(&self) {
        println!("{}", N * N);
        for i in 0..N {
            for j in 0..N {
                println!("1 {} {} U", i, j);
                println!("R 0 R 0");
            }
        }
        for _ in 0..N {
            println!("{}", "0".repeat(N - 1));
        }
        for _ in 0..N - 1 {
            println!("{}", "0".repeat(N));
        }
    }
}

fn main() {
    let mut solver = Solver::new();
    solver.solve();
}
