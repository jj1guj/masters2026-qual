use proconio::input;

const N: usize = 20;

// Directions: 0=Up, 1=Right, 2=Down, 3=Left
const DI: [i32; 4] = [-1, 0, 1, 0];
const DJ: [i32; 4] = [0, 1, 0, -1];

#[derive(Debug, Clone)]
enum CompressedState {
    SingleF,                  // A single F where front is not wall
    Turn(char),               // L or R turn
    RunF { wall_turn: char }, // "no wall → F(self), wall → turn(next)"
}

struct Solver {
    _n: usize,
    _a_k: i64,
    _a_m: i64,
    _a_w: i64,
    wall_v: Vec<Vec<u8>>, // wall_v[i][j] = wall between (i,j) and (i,j+1)
    wall_h: Vec<Vec<u8>>, // wall_h[i][j] = wall between (i,j) and (i+1,j)
}

impl Solver {
    fn new() -> Self {
        input! {
            n: usize,
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
            _n: n,
            _a_k: a_k,
            _a_m: a_m,
            _a_w: a_w,
            wall_v,
            wall_h,
        }
    }

    /// Check if movement from (i,j) in direction d is possible (no wall, in bounds)
    fn can_move(&self, i: usize, j: usize, d: usize) -> bool {
        match d {
            0 => i > 0 && self.wall_h[i - 1][j] == 0, // Up
            1 => j + 1 < N && self.wall_v[i][j] == 0, // Right
            2 => i + 1 < N && self.wall_h[i][j] == 0, // Down
            3 => j > 0 && self.wall_v[i][j - 1] == 0, // Left
            _ => unreachable!(),
        }
    }

    /// Build a DFS spanning tree and return Euler tour as position sequence.
    /// Direction preference creates a snake-like pattern to minimize turns.
    fn dfs_euler_tour(&self) -> Vec<(usize, usize)> {
        let mut visited = vec![vec![false; N]; N];
        let mut tour = Vec::new();
        self.dfs(0, 0, &mut visited, &mut tour);
        tour
    }

    fn dfs(
        &self,
        i: usize,
        j: usize,
        visited: &mut Vec<Vec<bool>>,
        tour: &mut Vec<(usize, usize)>,
    ) {
        visited[i][j] = true;
        tour.push((i, j));

        // Snake pattern: even rows prefer Right, odd rows prefer Left
        let dirs: [usize; 4] = if i % 2 == 0 {
            [1, 2, 0, 3] // Right, Down, Up, Left
        } else {
            [3, 2, 0, 1] // Left, Down, Up, Right
        };

        for &d in &dirs {
            if !self.can_move(i, j, d) {
                continue;
            }
            let ni = ((i as i32) + DI[d]) as usize;
            let nj = ((j as i32) + DJ[d]) as usize;
            if !visited[ni][nj] {
                self.dfs(ni, nj, visited, tour);
                tour.push((i, j)); // backtrack
            }
        }
    }

    /// Determine direction from (i1,j1) to adjacent (i2,j2)
    fn direction_between(i1: usize, j1: usize, i2: usize, j2: usize) -> usize {
        if i2 < i1 {
            0 // Up
        } else if j2 > j1 {
            1 // Right
        } else if i2 > i1 {
            2 // Down
        } else {
            3 // Left
        }
    }

    /// Return sequence of turns (L/R) needed to change from direction `from` to `to`
    fn turns_needed(from: usize, to: usize) -> Vec<char> {
        let diff = (to + 4 - from) % 4;
        match diff {
            0 => vec![],
            1 => vec!['R'],
            2 => vec!['R', 'R'], // U-turn
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

    /// Compress action sequence: merge consecutive F-runs ending at a wall into single states.
    /// A run of F's followed by a turn (L/R) where front_is_wall=true can be compressed:
    ///   F, F, ..., F, Turn(wall=true) → RunF{wall_turn}, then continue after the turn
    fn compress_actions(actions: &[(char, bool)]) -> Vec<CompressedState> {
        let mut result = Vec::new();
        let n = actions.len();
        let mut i = 0;

        while i < n {
            if actions[i].0 == 'F' {
                // Count consecutive F's
                let f_start = i;
                while i < n && actions[i].0 == 'F' {
                    i += 1;
                }
                let f_count = i - f_start;

                // Check if this F-run is followed by a turn with front_is_wall=true
                if i < n && actions[i].0 != 'F' && actions[i].1 {
                    // Can compress: all F's + the turn → one RunF state
                    let turn_char = actions[i].0;
                    result.push(CompressedState::RunF {
                        wall_turn: turn_char,
                    });
                    i += 1; // consume the turn
                } else {
                    // Cannot compress: emit F's individually
                    for _ in 0..f_count {
                        result.push(CompressedState::SingleF);
                    }
                }
            } else {
                // Turn action (L or R)
                result.push(CompressedState::Turn(actions[i].0));
                i += 1;
            }
        }

        // Handle wrap-around: if the sequence starts with F's and ends with F's,
        // check if the first action's run can merge with the last F run
        // (This is complex; for now, leave as-is since the loop closing turn handles it)

        result
    }

    fn solve(&mut self) {
        // 1. Build Euler tour of DFS spanning tree
        let tour = self.dfs_euler_tour();

        // 2. Convert position sequence to action sequence
        let init_dir = Self::direction_between(tour[0].0, tour[0].1, tour[1].0, tour[1].1);
        let mut actions: Vec<(char, bool)> = Vec::new(); // (action, front_is_wall)
        let mut cur_dir = init_dir;

        for step in 0..tour.len() - 1 {
            let (ci, cj) = tour[step];
            let (ni, nj) = tour[step + 1];
            let target_dir = Self::direction_between(ci, cj, ni, nj);

            // Generate turns to face target direction
            let turns = Self::turns_needed(cur_dir, target_dir);
            for &t in &turns {
                let front_wall = !self.can_move(ci, cj, cur_dir);
                actions.push((t, front_wall));
                cur_dir = Self::apply_turn(cur_dir, t);
            }

            // Move forward
            actions.push(('F', false));
        }

        // Close the loop: turn to face init_dir at (0,0)
        let (ci, cj) = tour[tour.len() - 1];
        let turns = Self::turns_needed(cur_dir, init_dir);
        for &t in &turns {
            let front_wall = !self.can_move(ci, cj, cur_dir);
            actions.push((t, front_wall));
            cur_dir = Self::apply_turn(cur_dir, t);
        }

        // 3. Compress consecutive F-runs that end at a wall into single states
        //    Pattern: F, F, ..., F, (L or R with front_wall=true)
        //    Compressed: one state "no wall → F(self), wall → turn(next)"
        let compressed = Self::compress_actions(&actions);
        let m = compressed.len();

        // 4. Fallback if state count exceeds limit (4*N*N = 1600)
        if m > 4 * N * N {
            eprintln!(
                "Warning: state count {} exceeds limit, using naive fallback",
                m
            );
            self.solve_naive();
            return;
        }

        // 5. Output
        println!("1"); // K = 1 robot
        println!(
            "{} {} {} {}",
            m,
            tour[0].0,
            tour[0].1,
            Self::dir_char(init_dir)
        );

        for (idx, state) in compressed.iter().enumerate() {
            let next_state = (idx + 1) % m;
            match state {
                CompressedState::SingleF => {
                    // F when no wall (expected), dummy R when wall (impossible)
                    println!("F {} R {}", next_state, next_state);
                }
                CompressedState::Turn(c) => {
                    // L or R: valid regardless of wall
                    println!("{} {} {} {}", c, next_state, c, next_state);
                }
                CompressedState::RunF { wall_turn } => {
                    // No wall → F (stay in this state), wall → turn (go to next state)
                    println!("F {} {} {}", idx, wall_turn, next_state);
                }
            }
        }

        // 6. No new walls
        for _ in 0..N {
            println!("{}", "0".repeat(N - 1));
        }
        for _ in 0..N - 1 {
            println!("{}", "0".repeat(N));
        }

        eprintln!(
            "States: {} (before compress: {}), Cost: {}",
            m,
            actions.len(),
            m
        );
    }

    fn solve_naive(&mut self) {
        let k = N * N;
        println!("{}", k);
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
