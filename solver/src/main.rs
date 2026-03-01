use proconio::{input, marker::Chars};

const MAX_N: usize = 20;
const K: usize = MAX_N * MAX_N;
const DX: [i32; 4] = [-1, 0, 0, 1];
const DY: [i32; 4] = [0, -1, 1, 0];

/// 6-state snake automaton
const SNAKE_AUTOMATON: [(u8, usize, u8, usize); 6] = [
    (0, 0, 1, 1),
    (0, 2, 1, 2),
    (1, 3, 1, 3),
    (0, 3, 2, 4),
    (0, 5, 2, 5),
    (2, 0, 2, 0),
];

/// 6-state reverse snake automaton (mirror: swaps R<->L)
const REVERSE_SNAKE_AUTOMATON: [(u8, usize, u8, usize); 6] = [
    (0, 0, 2, 1),
    (0, 2, 2, 2),
    (2, 3, 2, 3),
    (0, 3, 1, 4),
    (0, 5, 1, 5),
    (1, 0, 1, 0),
];

/// 2-state automaton A
const AUTO_2S_A: [(u8, usize, u8, usize); 2] = [(2, 1, 1, 1), (0, 0, 2, 1)];
/// Mirror of 2S_A
const AUTO_2S_B: [(u8, usize, u8, usize); 2] = [(0, 1, 2, 0), (2, 0, 1, 0)];

/// 2-state automaton C
const AUTO_2S_C: [(u8, usize, u8, usize); 2] = [(1, 1, 2, 1), (0, 0, 1, 1)];
/// Mirror of 2S_C
const AUTO_2S_D: [(u8, usize, u8, usize); 2] = [(0, 1, 1, 0), (1, 0, 2, 0)];

/// 3-state automaton A
const AUTO_3S_A: [(u8, usize, u8, usize); 3] = [(1, 1, 2, 2), (2, 2, 2, 1), (0, 0, 1, 2)];
/// Mirror of 3S_A
const AUTO_3S_B: [(u8, usize, u8, usize); 3] = [(2, 1, 1, 2), (1, 2, 1, 1), (0, 0, 2, 2)];

/// 5-state generic corridor automaton
const CORRIDOR_AUTOMATON: [(u8, usize, u8, usize); 5] = [
    (0, 0, 1, 1),
    (0, 0, 2, 2),
    (0, 0, 2, 3),
    (0, 0, 2, 4),
    (0, 0, 2, 0),
];

/// All automaton definitions: (table slice, num_states)
struct AutomatonDef {
    table: &'static [(u8, usize, u8, usize)],
}

const AUTOMATA: [AutomatonDef; 9] = [
    AutomatonDef {
        table: &SNAKE_AUTOMATON,
    },
    AutomatonDef {
        table: &REVERSE_SNAKE_AUTOMATON,
    },
    AutomatonDef { table: &AUTO_2S_A },
    AutomatonDef { table: &AUTO_2S_B },
    AutomatonDef { table: &AUTO_2S_C },
    AutomatonDef { table: &AUTO_2S_D },
    AutomatonDef { table: &AUTO_3S_A },
    AutomatonDef { table: &AUTO_3S_B },
    AutomatonDef {
        table: &CORRIDOR_AUTOMATON,
    },
];

struct Solver {
    n: usize,
    a_k: i64,
    a_m: i64,
    a_w: i64,
    v: [[bool; MAX_N - 1]; MAX_N],
    h: [[bool; MAX_N]; MAX_N - 1],
}

impl Solver {
    fn new() -> Self {
        input! {
            n: usize,
            a_k: i64,
            a_m: i64,
            a_w: i64,
            wall_v: [Chars; MAX_N],
            wall_h: [Chars; MAX_N -1],
        }

        let mut v = [[false; MAX_N - 1]; MAX_N];
        let mut h = [[false; MAX_N]; MAX_N - 1];
        for i in 0..MAX_N {
            for j in 0..MAX_N - 1 {
                if wall_v[i][j] == '1' {
                    v[i][j] = true;
                }
            }
        }

        for i in 0..MAX_N - 1 {
            for j in 0..MAX_N {
                if wall_h[i][j] == '1' {
                    h[i][j] = true;
                }
            }
        }

        Solver {
            n,
            a_k,
            a_m,
            a_w,
            v,
            h,
        }
    }

    fn dfs(
        &self,
        pos: (usize, usize),
        area_map: &mut [[i32; MAX_N]; MAX_N],
        v: &[[bool; MAX_N - 1]; MAX_N],
        h: &[[bool; MAX_N]; MAX_N - 1],
    ) {
        let (x, y) = pos;
        for i in 0..4 {
            let nx_i32 = x as i32 + DX[i];
            let ny_i32 = y as i32 + DY[i];
            if 0 <= nx_i32 && nx_i32 < MAX_N as i32 && 0 <= ny_i32 && ny_i32 < MAX_N as i32 {
                let nx = nx_i32 as usize;
                let ny = ny_i32 as usize;

                // Check for walls between (x,y) and (nx,ny)
                let blocked = match i {
                    0 => x > 0 && h[nx][y],        // up: h[x-1][y]
                    1 => y > 0 && v[x][ny],        // left: v[x][y-1]
                    2 => y < MAX_N - 1 && v[x][y], // right: v[x][y]
                    3 => x < MAX_N - 1 && h[x][y], // down: h[x][y]
                    _ => unreachable!(),
                };

                if !blocked && area_map[nx][ny] == -1 {
                    area_map[nx][ny] = area_map[x][y];
                    self.dfs((nx, ny), area_map, &v, &h);
                }
            }
        }
    }

    /// 縦壁を下方向に伸ばす
    fn extend_v_down(&mut self, v_out: &mut [[i32; MAX_N - 1]; MAX_N]) {
        for j in 0..MAX_N - 1 {
            for i in 0..MAX_N {
                if self.v[i][j] || v_out[i][j] == 1 {
                    for r in (i + 1)..MAX_N {
                        if r > 0 {
                            let hit = (j < MAX_N - 1 && self.h[r - 1][j + 1]) || self.h[r - 1][j];
                            if hit {
                                break;
                            }
                        }
                        if self.v[r][j] || v_out[r][j] == 1 {
                            break;
                        }
                        v_out[r][j] = 1;
                        self.v[r][j] = true;
                    }
                }
            }
        }
    }

    /// 縦壁を上方向に伸ばす
    fn extend_v_up(&mut self, v_out: &mut [[i32; MAX_N - 1]; MAX_N]) {
        for j in 0..MAX_N - 1 {
            for i in (0..MAX_N).rev() {
                if self.v[i][j] || v_out[i][j] == 1 {
                    for r in (0..i).rev() {
                        if r < MAX_N - 1 {
                            let hit = (j < MAX_N - 1 && self.h[r][j + 1]) || self.h[r][j];
                            if hit {
                                break;
                            }
                        }
                        if self.v[r][j] || v_out[r][j] == 1 {
                            break;
                        }
                        v_out[r][j] = 1;
                        self.v[r][j] = true;
                    }
                }
            }
        }
    }

    /// 横壁を右方向に伸ばす
    fn extend_h_right(
        &mut self,
        h_out: &mut [[i32; MAX_N]; MAX_N - 1],
        v_out: &[[i32; MAX_N - 1]; MAX_N],
    ) {
        for i in 0..MAX_N - 1 {
            for j in 0..MAX_N {
                if self.h[i][j] || h_out[i][j] == 1 {
                    for c in (j + 1)..MAX_N {
                        if c > 0 {
                            let hit = (i < MAX_N - 1
                                && (self.v[i + 1][c - 1] || v_out[i + 1][c - 1] == 1))
                                || (self.v[i][c - 1] || v_out[i][c - 1] == 1);
                            if hit {
                                break;
                            }
                        }
                        if self.h[i][c] || h_out[i][c] == 1 {
                            break;
                        }
                        h_out[i][c] = 1;
                        self.h[i][c] = true;
                    }
                }
            }
        }
    }

    /// 横壁を左方向に伸ばす
    fn extend_h_left(
        &mut self,
        h_out: &mut [[i32; MAX_N]; MAX_N - 1],
        v_out: &[[i32; MAX_N - 1]; MAX_N],
    ) {
        for i in 0..MAX_N - 1 {
            for j in (0..MAX_N).rev() {
                if self.h[i][j] || h_out[i][j] == 1 {
                    for c in (0..j).rev() {
                        if c < MAX_N - 1 {
                            let hit = (i < MAX_N - 1 && (self.v[i + 1][c] || v_out[i + 1][c] == 1))
                                || (self.v[i][c] || v_out[i][c] == 1);
                            if hit {
                                break;
                            }
                        }
                        if self.h[i][c] || h_out[i][c] == 1 {
                            break;
                        }
                        h_out[i][c] = 1;
                        self.h[i][c] = true;
                    }
                }
            }
        }
    }

    /// 壁延伸の1操作を適用する (0=V↓, 1=V↑, 2=H→, 3=H←)
    fn apply_extend_op(
        &mut self,
        op: usize,
        v_out: &mut [[i32; MAX_N - 1]; MAX_N],
        h_out: &mut [[i32; MAX_N]; MAX_N - 1],
    ) {
        match op {
            0 => self.extend_v_down(v_out),
            1 => self.extend_v_up(v_out),
            2 => self.extend_h_right(h_out, v_out),
            3 => self.extend_h_left(h_out, v_out),
            _ => unreachable!(),
        }
    }

    /// 複数の壁延伸戦略を試し、領域数が最小となるものを選択する
    fn extend_walls(&mut self) -> ([[i32; MAX_N - 1]; MAX_N], [[i32; MAX_N]; MAX_N - 1]) {
        let orig_v = self.v;
        let orig_h = self.h;

        let mut best_v = self.v;
        let mut best_h = self.h;
        let mut best_v_out = [[0i32; MAX_N - 1]; MAX_N];
        let mut best_h_out = [[0i32; MAX_N]; MAX_N - 1];
        let mut best_regions = i32::MAX;

        // 全24通りの延伸順列 + rectify-only を試す
        // Operations: 0=V↓, 1=V↑, 2=H→, 3=H←
        let mut strategies: Vec<Vec<usize>> = Vec::new();

        // All 24 permutations of 4 operations
        for a in 0..4usize {
            for b in 0..4usize {
                if b == a {
                    continue;
                }
                for c in 0..4usize {
                    if c == a || c == b {
                        continue;
                    }
                    let d = 6 - a - b - c;
                    strategies.push(vec![a, b, c, d]);
                }
            }
        }

        // Rectify-only (no extension)
        strategies.push(vec![]);

        for ops in &strategies {
            self.v = orig_v;
            self.h = orig_h;
            let mut v_out = [[0i32; MAX_N - 1]; MAX_N];
            let mut h_out = [[0i32; MAX_N]; MAX_N - 1];

            for &op in ops {
                self.apply_extend_op(op, &mut v_out, &mut h_out);
            }

            self.rectify(&mut v_out, &mut h_out);

            let (_, num_regions) = self.build_area_map();
            if num_regions < best_regions {
                best_regions = num_regions;
                best_v = self.v;
                best_h = self.h;
                best_v_out = v_out;
                best_h_out = h_out;
            }
        }

        self.v = best_v;
        self.h = best_h;
        (best_v_out, best_h_out)
    }

    /// 隣接セルの壁パターンの不整合を解消し、全領域を長方形にする
    fn rectify(
        &mut self,
        v_out: &mut [[i32; MAX_N - 1]; MAX_N],
        h_out: &mut [[i32; MAX_N]; MAX_N - 1],
    ) {
        loop {
            let mut changed = false;

            // 横方向: (i,j) と (i,j+1) の上下壁パターンが異なれば縦壁を追加
            for i in 0..MAX_N {
                for j in 0..MAX_N - 1 {
                    if self.v[i][j] {
                        continue;
                    }
                    let mut need_wall = false;
                    if i > 0 && self.h[i - 1][j] != self.h[i - 1][j + 1] {
                        need_wall = true;
                    }
                    if i < MAX_N - 1 && self.h[i][j] != self.h[i][j + 1] {
                        need_wall = true;
                    }
                    if need_wall {
                        self.v[i][j] = true;
                        v_out[i][j] = 1;
                        changed = true;
                    }
                }
            }

            // 縦方向: (i,j) と (i+1,j) の左右壁パターンが異なれば横壁を追加
            for i in 0..MAX_N - 1 {
                for j in 0..MAX_N {
                    if self.h[i][j] {
                        continue;
                    }
                    let mut need_wall = false;
                    if j > 0 && self.v[i][j - 1] != self.v[i + 1][j - 1] {
                        need_wall = true;
                    }
                    if j < MAX_N - 1 && self.v[i][j] != self.v[i + 1][j] {
                        need_wall = true;
                    }
                    if need_wall {
                        self.h[i][j] = true;
                        h_out[i][j] = 1;
                        changed = true;
                    }
                }
            }

            if !changed {
                break;
            }
        }
    }

    /// 指定位置・方向に壁があるかチェック (0=U, 1=R, 2=D, 3=L)
    fn has_wall_ahead_dir(&self, row: usize, col: usize, dir: usize) -> bool {
        match dir {
            0 => row == 0 || self.h[row - 1][col],
            1 => col == MAX_N - 1 || self.v[row][col],
            2 => row == MAX_N - 1 || self.h[row][col],
            3 => col == 0 || self.v[row][col - 1],
            _ => unreachable!(),
        }
    }

    /// 任意のオートマトンでシミュレーションし、周期的行動中に訪れるマスを返す
    fn simulate_reachable_with(
        &self,
        start: (usize, usize),
        start_dir: usize,
        auto_table: &[(u8, usize, u8, usize)],
    ) -> [[bool; MAX_N]; MAX_N] {
        let num_states = auto_table.len();
        // visited: row x col x dir x state (max 6 states)
        let mut visited = vec![vec![vec![vec![false; num_states]; 4]; MAX_N]; MAX_N];
        let mut history: Vec<(usize, usize, usize, usize)> = Vec::new();

        let (mut row, mut col) = start;
        let mut dir = start_dir;
        let mut auto_state = 0usize;

        loop {
            if visited[row][col][dir][auto_state] {
                let target = (row, col, dir, auto_state);
                let cycle_start = history.iter().position(|s| *s == target).unwrap();
                let mut reachable = [[false; MAX_N]; MAX_N];
                for i in cycle_start..history.len() {
                    reachable[history[i].0][history[i].1] = true;
                }
                return reachable;
            }

            visited[row][col][dir][auto_state] = true;
            history.push((row, col, dir, auto_state));

            let wall = self.has_wall_ahead_dir(row, col, dir);
            let (action, next_state) = if wall {
                (auto_table[auto_state].2, auto_table[auto_state].3)
            } else {
                (auto_table[auto_state].0, auto_table[auto_state].1)
            };

            match action {
                0 => match dir {
                    0 => row -= 1,
                    1 => col += 1,
                    2 => row += 1,
                    3 => col -= 1,
                    _ => unreachable!(),
                },
                1 => dir = (dir + 1) % 4,
                2 => dir = (dir + 3) % 4,
                _ => unreachable!(),
            }
            auto_state = next_state;
        }
    }

    fn has_wall_ahead_dir_on(
        row: usize,
        col: usize,
        dir: usize,
        v: &[[bool; MAX_N - 1]; MAX_N],
        h: &[[bool; MAX_N]; MAX_N - 1],
    ) -> bool {
        match dir {
            0 => row == 0 || h[row - 1][col],
            1 => col == MAX_N - 1 || v[row][col],
            2 => row == MAX_N - 1 || h[row][col],
            3 => col == 0 || v[row][col - 1],
            _ => unreachable!(),
        }
    }

    fn simulate_reachable_with_board(
        &self,
        start: (usize, usize),
        start_dir: usize,
        auto_table: &[(u8, usize, u8, usize)],
        v: &[[bool; MAX_N - 1]; MAX_N],
        h: &[[bool; MAX_N]; MAX_N - 1],
    ) -> [[bool; MAX_N]; MAX_N] {
        let num_states = auto_table.len();
        let mut visited = vec![vec![vec![vec![false; num_states]; 4]; MAX_N]; MAX_N];
        let mut history: Vec<(usize, usize, usize, usize)> = Vec::new();

        let (mut row, mut col) = start;
        let mut dir = start_dir;
        let mut auto_state = 0usize;

        loop {
            if visited[row][col][dir][auto_state] {
                let target = (row, col, dir, auto_state);
                let cycle_start = history.iter().position(|s| *s == target).unwrap();
                let mut reachable = [[false; MAX_N]; MAX_N];
                for i in cycle_start..history.len() {
                    reachable[history[i].0][history[i].1] = true;
                }
                return reachable;
            }

            visited[row][col][dir][auto_state] = true;
            history.push((row, col, dir, auto_state));

            let wall = Self::has_wall_ahead_dir_on(row, col, dir, v, h);
            let (action, next_state) = if wall {
                (auto_table[auto_state].2, auto_table[auto_state].3)
            } else {
                (auto_table[auto_state].0, auto_table[auto_state].1)
            };

            match action {
                0 => match dir {
                    0 => row -= 1,
                    1 => col += 1,
                    2 => row += 1,
                    3 => col -= 1,
                    _ => unreachable!(),
                },
                1 => dir = (dir + 1) % 4,
                2 => dir = (dir + 3) % 4,
                _ => unreachable!(),
            }
            auto_state = next_state;
        }
    }

    fn find_valid_dir_on_board(
        &self,
        start: (usize, usize),
        auto_idx: usize,
        v: &[[bool; MAX_N - 1]; MAX_N],
        h: &[[bool; MAX_N]; MAX_N - 1],
    ) -> Option<usize> {
        for dir in 0..4 {
            let reachable =
                self.simulate_reachable_with_board(start, dir, AUTOMATA[auto_idx].table, v, h);
            if reachable.iter().all(|row| row.iter().all(|&x| x)) {
                return Some(dir);
            }
        }
        None
    }

    fn optimize_single_robot_walls(
        &self,
        start: (usize, usize),
        auto_idx: usize,
        init_dir: usize,
        v_out: &mut [[i32; MAX_N - 1]; MAX_N],
        h_out: &mut [[i32; MAX_N]; MAX_N - 1],
    ) -> usize {
        let mut board_v = self.v;
        let mut board_h = self.h;
        for i in 0..MAX_N {
            for j in 0..MAX_N - 1 {
                if v_out[i][j] == 1 {
                    board_v[i][j] = true;
                }
            }
        }
        for i in 0..MAX_N - 1 {
            for j in 0..MAX_N {
                if h_out[i][j] == 1 {
                    board_h[i][j] = true;
                }
            }
        }

        let mut candidates: Vec<(bool, usize, usize)> = Vec::new();
        for i in 0..MAX_N {
            for j in 0..MAX_N - 1 {
                if v_out[i][j] == 1 {
                    candidates.push((true, i, j));
                }
            }
        }
        for i in 0..MAX_N - 1 {
            for j in 0..MAX_N {
                if h_out[i][j] == 1 {
                    candidates.push((false, i, j));
                }
            }
        }

        let mut seed = 0xA24BAED4963EE407u64
            ^ ((start.0 as u64) << 24)
            ^ ((start.1 as u64) << 16)
            ^ (self.a_w as u64)
            ^ ((candidates.len() as u64) << 32);
        let begin = std::time::Instant::now();
        let time_limit = std::time::Duration::from_millis(120);
        let mut best_dir = init_dir;

        for _ in 0..2 {
            if begin.elapsed() > time_limit {
                break;
            }

            let mut improved = false;
            let mut order: Vec<usize> = (0..candidates.len()).collect();
            Self::shuffle_vec(&mut order, &mut seed);

            for idx in order {
                if begin.elapsed() > time_limit {
                    break;
                }
                let (is_v, i, j) = candidates[idx];
                if is_v {
                    if v_out[i][j] == 0 {
                        continue;
                    }
                    v_out[i][j] = 0;
                    board_v[i][j] = false;
                } else {
                    if h_out[i][j] == 0 {
                        continue;
                    }
                    h_out[i][j] = 0;
                    board_h[i][j] = false;
                }

                if let Some(new_dir) = self.find_valid_dir_on_board(start, auto_idx, &board_v, &board_h)
                {
                    best_dir = new_dir;
                    improved = true;
                } else if is_v {
                    v_out[i][j] = 1;
                    board_v[i][j] = true;
                } else {
                    h_out[i][j] = 1;
                    board_h[i][j] = true;
                }
            }

            if !improved {
                break;
            }
        }

        best_dir
    }

    fn cell_idx(row: usize, col: usize) -> usize {
        row * MAX_N + col
    }

    fn idx_cell(idx: usize) -> (usize, usize) {
        (idx / MAX_N, idx % MAX_N)
    }

    fn open_neighbors(&self, row: usize, col: usize) -> Vec<usize> {
        let mut res = Vec::with_capacity(4);
        if row > 0 && !self.h[row - 1][col] {
            res.push(Self::cell_idx(row - 1, col));
        }
        if col + 1 < MAX_N && !self.v[row][col] {
            res.push(Self::cell_idx(row, col + 1));
        }
        if row + 1 < MAX_N && !self.h[row][col] {
            res.push(Self::cell_idx(row + 1, col));
        }
        if col > 0 && !self.v[row][col - 1] {
            res.push(Self::cell_idx(row, col - 1));
        }
        res
    }

    fn build_open_graph(&self) -> Vec<Vec<usize>> {
        let mut adj = vec![Vec::new(); MAX_N * MAX_N];
        for row in 0..MAX_N {
            for col in 0..MAX_N {
                adj[Self::cell_idx(row, col)] = self.open_neighbors(row, col);
            }
        }
        adj
    }

    fn xorshift64(seed: &mut u64) -> u64 {
        *seed ^= *seed << 7;
        *seed ^= *seed >> 9;
        *seed
    }

    fn shuffle_vec(v: &mut [usize], seed: &mut u64) {
        for i in (1..v.len()).rev() {
            let r = (Self::xorshift64(seed) as usize) % (i + 1);
            v.swap(i, r);
        }
    }

    fn prune_unvisited_connected(
        &self,
        adj: &Vec<Vec<usize>>,
        visited: &[bool; MAX_N * MAX_N],
        visited_count: usize,
    ) -> bool {
        let rem = MAX_N * MAX_N - visited_count;
        if rem == 0 {
            return true;
        }

        let mut start = None;
        for i in 0..(MAX_N * MAX_N) {
            if !visited[i] {
                start = Some(i);
                break;
            }
        }
        let Some(start_idx) = start else {
            return true;
        };

        let mut q = std::collections::VecDeque::new();
        let mut seen = [false; MAX_N * MAX_N];
        seen[start_idx] = true;
        q.push_back(start_idx);
        let mut count = 1usize;

        while let Some(u) = q.pop_front() {
            for &v in &adj[u] {
                if !visited[v] && !seen[v] {
                    seen[v] = true;
                    q.push_back(v);
                    count += 1;
                }
            }
        }

        if count != rem {
            return false;
        }

        if rem > 1 {
            for u in 0..(MAX_N * MAX_N) {
                if visited[u] {
                    continue;
                }
                let mut deg = 0usize;
                for &v in &adj[u] {
                    if !visited[v] {
                        deg += 1;
                    }
                }
                if deg == 0 {
                    return false;
                }
            }
        }

        true
    }

    fn dfs_hamilton(
        &self,
        adj: &Vec<Vec<usize>>,
        current: usize,
        visited: &mut [bool; MAX_N * MAX_N],
        path: &mut Vec<usize>,
        visited_count: usize,
        seed: &mut u64,
        start_time: std::time::Instant,
        limit: std::time::Duration,
    ) -> bool {
        if visited_count == MAX_N * MAX_N {
            return true;
        }
        if start_time.elapsed() > limit {
            return false;
        }

        let mut candidates: Vec<usize> = adj[current]
            .iter()
            .copied()
            .filter(|&nxt| !visited[nxt])
            .collect();
        if candidates.is_empty() {
            return false;
        }

        Self::shuffle_vec(&mut candidates, seed);
        candidates.sort_by_key(|&nxt| {
            adj[nxt]
                .iter()
                .filter(|&&to| !visited[to] && to != current)
                .count()
        });

        for nxt in candidates {
            visited[nxt] = true;
            path.push(nxt);

            if self.prune_unvisited_connected(adj, visited, visited_count + 1)
                && self.dfs_hamilton(
                    adj,
                    nxt,
                    visited,
                    path,
                    visited_count + 1,
                    seed,
                    start_time,
                    limit,
                )
            {
                return true;
            }

            path.pop();
            visited[nxt] = false;
        }

        false
    }

    fn greedy_hamilton(
        &self,
        adj: &Vec<Vec<usize>>,
        start: usize,
        seed: &mut u64,
    ) -> Option<Vec<usize>> {
        let mut visited = [false; MAX_N * MAX_N];
        let mut path = Vec::with_capacity(MAX_N * MAX_N);
        let mut current = start;
        visited[current] = true;
        path.push(current);

        for _ in 1..(MAX_N * MAX_N) {
            let mut candidates: Vec<usize> = adj[current]
                .iter()
                .copied()
                .filter(|&to| !visited[to])
                .collect();
            if candidates.is_empty() {
                return None;
            }

            Self::shuffle_vec(&mut candidates, seed);
            candidates.sort_by_key(|&to| {
                let onward = adj[to].iter().filter(|&&n2| !visited[n2] && n2 != current).count();
                let second = adj[to]
                    .iter()
                    .filter(|&&n2| !visited[n2] && n2 != current)
                    .map(|&n2| adj[n2].iter().filter(|&&n3| !visited[n3] && n3 != to).count())
                    .min()
                    .unwrap_or(usize::MAX);
                (onward, second)
            });

            let nxt = candidates[0];
            visited[nxt] = true;
            path.push(nxt);
            current = nxt;
        }

        Some(path)
    }

    fn find_hamilton_path(&self) -> Option<Vec<(usize, usize)>> {
        let adj = self.build_open_graph();
        let mut starts: Vec<usize> = (0..MAX_N * MAX_N)
            .filter(|&u| !adj[u].is_empty())
            .collect();
        if starts.is_empty() {
            return None;
        }
        starts.sort_by_key(|&u| adj[u].len());

        let start_time = std::time::Instant::now();
        let total_limit = if self.a_k >= 500 {
            std::time::Duration::from_millis(900)
        } else {
            std::time::Duration::from_millis(450)
        };

        let seed_base = 0x9E3779B97F4A7C15u64
            ^ (self.a_k as u64)
            ^ ((self.a_m as u64) << 16)
            ^ ((self.a_w as u64) << 40);

        'outer: for (rank, &s) in starts.iter().enumerate() {
            if start_time.elapsed() > total_limit {
                break;
            }

            // まず高速な greedy 構築を複数回試す
            let greedy_trials = if self.a_k >= 500 { 20usize } else { 8usize };
            for g in 0..greedy_trials {
                if start_time.elapsed() > total_limit {
                    break 'outer;
                }
                let mut seed = seed_base
                    ^ ((s as u64) << 32)
                    ^ ((rank as u64) << 8)
                    ^ ((g as u64) << 48)
                    ^ 0xC6A4A7935BD1E995u64;
                if let Some(path) = self.greedy_hamilton(&adj, s, &mut seed) {
                    let route = path.into_iter().map(Self::idx_cell).collect::<Vec<_>>();
                    return Some(route);
                }
            }

            let retries = if adj[s].len() <= 2 { 7usize } else { 4usize };
            for attempt in 0..retries {
                if start_time.elapsed() > total_limit {
                    break 'outer;
                }

                let mut seed = seed_base
                    ^ ((s as u64) << 32)
                    ^ ((rank as u64) << 8)
                    ^ (attempt as u64);

                let mut visited = [false; MAX_N * MAX_N];
                visited[s] = true;
                let mut path = vec![s];

                if self.dfs_hamilton(
                    &adj,
                    s,
                    &mut visited,
                    &mut path,
                    1,
                    &mut seed,
                    start_time,
                    total_limit,
                ) {
                    let route = path.into_iter().map(Self::idx_cell).collect::<Vec<_>>();
                    return Some(route);
                }
            }
        }

        None
    }

    fn build_walls_for_route(
        &self,
        route: &[(usize, usize)],
    ) -> ([[i32; MAX_N - 1]; MAX_N], [[i32; MAX_N]; MAX_N - 1]) {
        let mut keep_v = [[false; MAX_N - 1]; MAX_N];
        let mut keep_h = [[false; MAX_N]; MAX_N - 1];

        for w in route.windows(2) {
            let (r1, c1) = w[0];
            let (r2, c2) = w[1];
            if r1 == r2 {
                let j = c1.min(c2);
                keep_v[r1][j] = true;
            } else {
                let i = r1.min(r2);
                keep_h[i][c1] = true;
            }
        }

        let mut v_out = [[0i32; MAX_N - 1]; MAX_N];
        let mut h_out = [[0i32; MAX_N]; MAX_N - 1];

        for i in 0..MAX_N {
            for j in 0..MAX_N - 1 {
                if !keep_v[i][j] && !self.v[i][j] {
                    v_out[i][j] = 1;
                }
            }
        }
        for i in 0..MAX_N - 1 {
            for j in 0..MAX_N {
                if !keep_h[i][j] && !self.h[i][j] {
                    h_out[i][j] = 1;
                }
            }
        }

        (v_out, h_out)
    }

    fn route_initial_dir(route: &[(usize, usize)]) -> usize {
        if route.len() < 2 {
            return 1;
        }
        let (r0, c0) = route[0];
        let (r1, c1) = route[1];
        if r1 + 1 == r0 {
            0
        } else if c1 == c0 + 1 {
            1
        } else if r1 == r0 + 1 {
            2
        } else {
            3
        }
    }

    fn try_single_robot_route(
        &self,
    ) -> Option<(
        Vec<((usize, usize), usize, usize)>,
        [[i32; MAX_N - 1]; MAX_N],
        [[i32; MAX_N]; MAX_N - 1],
    )> {
        let route = self.find_hamilton_path()?;
        let (v_out, h_out) = self.build_walls_for_route(&route);

        let start = route[0];
        let corridor_idx = AUTOMATA.len() - 1;

        let mut board_v = self.v;
        let mut board_h = self.h;
        for i in 0..MAX_N {
            for j in 0..MAX_N - 1 {
                if v_out[i][j] == 1 {
                    board_v[i][j] = true;
                }
            }
        }
        for i in 0..MAX_N - 1 {
            for j in 0..MAX_N {
                if h_out[i][j] == 1 {
                    board_h[i][j] = true;
                }
            }
        }

        let init_dir = Self::route_initial_dir(&route);
        let dir = self
            .find_valid_dir_on_board(start, corridor_idx, &board_v, &board_h)
            .or(Some(init_dir))?;

        let configs = vec![(start, dir, corridor_idx)];
        Some((configs, v_out, h_out))
    }

    /// 現在の壁状態でエリアマップを構築する（0-indexed）
    fn build_area_map(&self) -> ([[i32; MAX_N]; MAX_N], i32) {
        let mut area_map = [[-1i32; MAX_N]; MAX_N];
        let mut count = 0i32;
        for i in 0..MAX_N {
            for j in 0..MAX_N {
                if area_map[i][j] == -1 {
                    area_map[i][j] = count;
                    self.dfs((i, j), &mut area_map, &self.v, &self.h);
                    count += 1;
                }
            }
        }
        (area_map, count)
    }

    /// 全オートマトンで開始位置・方向を試し、全セルをカバーできる構成があれば返す
    fn try_cover_cells(
        &self,
        cells: &[(usize, usize)],
        starts: &[(usize, usize)],
    ) -> Option<((usize, usize), usize, usize)> {
        for (auto_idx, auto_def) in AUTOMATA.iter().enumerate() {
            for &start in starts {
                for dir in 0..4 {
                    let reachable = self.simulate_reachable_with(start, dir, auto_def.table);
                    if cells.iter().all(|&(r, c)| reachable[r][c]) {
                        return Some((start, dir, auto_idx));
                    }
                }
            }
        }
        None
    }

    /// 設置した壁を破壊して隣接領域をマージする試行を行う
    fn try_merge_regions(
        &mut self,
        v_out: &mut [[i32; MAX_N - 1]; MAX_N],
        h_out: &mut [[i32; MAX_N]; MAX_N - 1],
    ) {
        loop {
            let mut merged_any = false;
            let (area_map, num_regions) = self.build_area_map();

            // 各領域のセルを収集
            let mut region_cells: Vec<Vec<(usize, usize)>> = vec![Vec::new(); num_regions as usize];
            for i in 0..MAX_N {
                for j in 0..MAX_N {
                    region_cells[area_map[i][j] as usize].push((i, j));
                }
            }

            // 追加壁で隣接する領域ペアと、その間の壁を収集
            let mut pairs: Vec<(i32, i32, Vec<(bool, usize, usize)>)> = Vec::new();

            for i in 0..MAX_N {
                for j in 0..MAX_N - 1 {
                    if v_out[i][j] == 1 {
                        let a = area_map[i][j];
                        let b = area_map[i][j + 1];
                        if a != b {
                            let pair = (a.min(b), a.max(b));
                            if let Some(entry) =
                                pairs.iter_mut().find(|(pa, pb, _)| (*pa, *pb) == pair)
                            {
                                entry.2.push((true, i, j));
                            } else {
                                pairs.push((pair.0, pair.1, vec![(true, i, j)]));
                            }
                        }
                    }
                }
            }
            for i in 0..MAX_N - 1 {
                for j in 0..MAX_N {
                    if h_out[i][j] == 1 {
                        let a = area_map[i][j];
                        let b = area_map[i + 1][j];
                        if a != b {
                            let pair = (a.min(b), a.max(b));
                            if let Some(entry) =
                                pairs.iter_mut().find(|(pa, pb, _)| (*pa, *pb) == pair)
                            {
                                entry.2.push((false, i, j));
                            } else {
                                pairs.push((pair.0, pair.1, vec![(false, i, j)]));
                            }
                        }
                    }
                }
            }

            for (ra, rb, walls) in &pairs {
                // 壁を一時的に除去
                for &(is_v, i, j) in walls {
                    if is_v {
                        self.v[i][j] = false;
                    } else {
                        self.h[i][j] = false;
                    }
                }

                let cells_a = &region_cells[*ra as usize];
                let cells_b = &region_cells[*rb as usize];
                let mut merged_cells: Vec<(usize, usize)> = Vec::new();
                merged_cells.extend_from_slice(cells_a);
                merged_cells.extend_from_slice(cells_b);

                let starts = [cells_a[0], cells_b[0]];
                let result = self.try_cover_cells(&merged_cells, &starts);

                if result.is_some() {
                    // マージ成功: v_out/h_out を更新
                    for &(is_v, i, j) in walls {
                        if is_v {
                            v_out[i][j] = 0;
                        } else {
                            h_out[i][j] = 0;
                        }
                    }
                    merged_any = true;
                    break;
                } else {
                    // マージ失敗: 壁を復元
                    for &(is_v, i, j) in walls {
                        if is_v {
                            self.v[i][j] = true;
                        } else {
                            self.h[i][j] = true;
                        }
                    }
                }
            }

            if !merged_any {
                break;
            }
        }
    }

    /// 3領域以上の同時マージを試行する。マージが1回でも成功したらtrueを返す。
    fn try_merge_multi_regions(
        &mut self,
        v_out: &mut [[i32; MAX_N - 1]; MAX_N],
        h_out: &mut [[i32; MAX_N]; MAX_N - 1],
    ) -> bool {
        let (area_map, num_regions) = self.build_area_map();
        let nr = num_regions as usize;

        // 各領域のセルを収集
        let mut region_cells: Vec<Vec<(usize, usize)>> = vec![Vec::new(); nr];
        for i in 0..MAX_N {
            for j in 0..MAX_N {
                region_cells[area_map[i][j] as usize].push((i, j));
            }
        }

        // 追加壁で隣接する領域ペアの壁情報を収集
        let mut pair_walls: std::collections::BTreeMap<(usize, usize), Vec<(bool, usize, usize)>> =
            std::collections::BTreeMap::new();

        for i in 0..MAX_N {
            for j in 0..MAX_N - 1 {
                if v_out[i][j] == 1 {
                    let a = area_map[i][j] as usize;
                    let b = area_map[i][j + 1] as usize;
                    if a != b {
                        let key = (a.min(b), a.max(b));
                        pair_walls.entry(key).or_default().push((true, i, j));
                    }
                }
            }
        }
        for i in 0..MAX_N - 1 {
            for j in 0..MAX_N {
                if h_out[i][j] == 1 {
                    let a = area_map[i][j] as usize;
                    let b = area_map[i + 1][j] as usize;
                    if a != b {
                        let key = (a.min(b), a.max(b));
                        pair_walls.entry(key).or_default().push((false, i, j));
                    }
                }
            }
        }

        // 隣接リスト構築
        let mut adj: Vec<Vec<usize>> = vec![Vec::new(); nr];
        for &(a, b) in pair_walls.keys() {
            adj[a].push(b);
            adj[b].push(a);
        }
        for list in &mut adj {
            list.sort();
            list.dedup();
        }

        // BFS で連結部分集合を列挙（サイズ3〜max_group_size）
        let max_group_size = 6usize;
        let max_attempts = 5000usize;
        let mut tried: std::collections::HashSet<Vec<usize>> = std::collections::HashSet::new();
        let mut attempts = 0usize;

        for start_r in 0..nr {
            if adj[start_r].is_empty() {
                continue;
            }
            let mut queue: std::collections::VecDeque<Vec<usize>> =
                std::collections::VecDeque::new();
            queue.push_back(vec![start_r]);

            while let Some(group) = queue.pop_front() {
                if attempts >= max_attempts {
                    return false;
                }

                if group.len() >= 3 {
                    let mut key = group.clone();
                    key.sort();
                    if tried.insert(key.clone()) {
                        attempts += 1;

                        // グループ内の全ペア間の追加壁を収集
                        let mut walls_to_remove: Vec<(bool, usize, usize)> = Vec::new();
                        for gi in 0..key.len() {
                            for gj in (gi + 1)..key.len() {
                                if let Some(ws) = pair_walls.get(&(key[gi], key[gj])) {
                                    walls_to_remove.extend_from_slice(ws);
                                }
                            }
                        }

                        if !walls_to_remove.is_empty() {
                            // 壁を一時除去
                            for &(is_v, i, j) in &walls_to_remove {
                                if is_v {
                                    self.v[i][j] = false;
                                } else {
                                    self.h[i][j] = false;
                                }
                            }

                            // 全セル・開始位置を収集
                            let mut merged_cells: Vec<(usize, usize)> = Vec::new();
                            let mut starts: Vec<(usize, usize)> = Vec::new();
                            for &r in &key {
                                merged_cells.extend_from_slice(&region_cells[r]);
                                starts.push(region_cells[r][0]);
                            }

                            let result = self.try_cover_cells(&merged_cells, &starts);

                            if result.is_some() {
                                // マージ成功: 壁を除去確定
                                for &(is_v, i, j) in &walls_to_remove {
                                    if is_v {
                                        v_out[i][j] = 0;
                                    } else {
                                        h_out[i][j] = 0;
                                    }
                                }
                                return true;
                            } else {
                                // 壁を復元
                                for &(is_v, i, j) in &walls_to_remove {
                                    if is_v {
                                        self.v[i][j] = true;
                                    } else {
                                        self.h[i][j] = true;
                                    }
                                }
                            }
                        }
                    }
                }

                if group.len() >= max_group_size {
                    continue;
                }

                // グループの隣接領域を追加（start_r より大きいもののみで重複排除）
                let mut neighbors: Vec<usize> = Vec::new();
                for &r in &group {
                    for &nb in &adj[r] {
                        if nb > start_r && !group.contains(&nb) && !neighbors.contains(&nb) {
                            neighbors.push(nb);
                        }
                    }
                }
                neighbors.sort();

                for nb in neighbors {
                    let mut new_group = group.clone();
                    new_group.push(nb);
                    queue.push_back(new_group);
                }
            }
        }

        false
    }

    /// 各領域のロボット配置位置・方向・オートマトンを決定する
    /// 戻り値: ((row, col), dir, automaton_index)
    fn find_robot_configs(&self) -> Vec<((usize, usize), usize, usize)> {
        let (area_map, num_regions) = self.build_area_map();
        let mut configs = Vec::new();

        for region_id in 0..num_regions {
            let mut cells: Vec<(usize, usize)> = Vec::new();
            for i in 0..MAX_N {
                for j in 0..MAX_N {
                    if area_map[i][j] == region_id {
                        cells.push((i, j));
                    }
                }
            }

            // まず左上セルで全オートマトン・全方向を試す
            let start = cells[0];
            if let Some(config) = self.try_cover_cells(&cells, &[start]) {
                configs.push(config);
                continue;
            }

            // 見つからなければ他のセルも試す
            if let Some(config) = self.try_cover_cells(&cells, &cells) {
                configs.push(config);
            } else {
                // フォールバック: snake automaton, right
                configs.push((start, 1, 0));
            }
        }

        configs
    }

    /// オートマトンテーブルを出力形式の文字列に変換
    fn format_automaton(table: &[(u8, usize, u8, usize)]) -> Vec<String> {
        let action_char = |a: u8| -> char {
            match a {
                0 => 'F',
                1 => 'R',
                2 => 'L',
                _ => unreachable!(),
            }
        };
        table
            .iter()
            .map(|&(a0, b0, a1, b1)| {
                format!("{} {} {} {}", action_char(a0), b0, action_char(a1), b1)
            })
            .collect()
    }

    /// ロボットと壁の情報を出力する
    fn output(
        &self,
        configs: &[((usize, usize), usize, usize)],
        v_out: &[[i32; MAX_N - 1]; MAX_N],
        h_out: &[[i32; MAX_N]; MAX_N - 1],
    ) {
        let dir_chars = ['U', 'R', 'D', 'L'];
        println!("{}", configs.len());
        for &((row, col), dir, auto_idx) in configs {
            let table = AUTOMATA[auto_idx].table;
            let num_states = table.len();
            println!("{} {} {} {}", num_states, row, col, dir_chars[dir]);
            for line in Self::format_automaton(table) {
                println!("{}", line);
            }
        }

        for i in 0..MAX_N {
            for j in 0..MAX_N - 1 {
                print!("{}", v_out[i][j]);
            }
            println!();
        }
        for i in 0..MAX_N - 1 {
            for j in 0..MAX_N {
                print!("{}", h_out[i][j]);
            }
            println!();
        }
    }

    fn solution_cost(
        &self,
        configs: &[((usize, usize), usize, usize)],
        v_out: &[[i32; MAX_N - 1]; MAX_N],
        h_out: &[[i32; MAX_N]; MAX_N - 1],
    ) -> i64 {
        let k = configs.len() as i64;
        let m = configs
            .iter()
            .map(|&(_, _, auto_idx)| AUTOMATA[auto_idx].table.len() as i64)
            .sum::<i64>();
        let mut w = 0i64;
        for i in 0..MAX_N {
            for j in 0..MAX_N - 1 {
                if v_out[i][j] == 1 {
                    w += 1;
                }
            }
        }
        for i in 0..MAX_N - 1 {
            for j in 0..MAX_N {
                if h_out[i][j] == 1 {
                    w += 1;
                }
            }
        }
        self.a_k * (k - 1) + self.a_m * m + self.a_w * w
    }

    fn solve_multi_robot(
        &mut self,
    ) -> (
        Vec<((usize, usize), usize, usize)>,
        [[i32; MAX_N - 1]; MAX_N],
        [[i32; MAX_N]; MAX_N - 1],
    ) {
        let (mut v_out, mut h_out) = self.extend_walls();
        loop {
            self.try_merge_regions(&mut v_out, &mut h_out);
            if !self.try_merge_multi_regions(&mut v_out, &mut h_out) {
                break;
            }
        }
        let configs = self.find_robot_configs();
        (configs, v_out, h_out)
    }

    fn solve(&mut self) {
        let mut single = self.try_single_robot_route();
        let multi = self.solve_multi_robot();

        if let Some((ref mut s_cfg, ref mut s_v, ref mut s_h)) = single {
            if s_cfg.len() == 1 {
                let (start, dir, auto_idx) = s_cfg[0];
                let new_dir = self.optimize_single_robot_walls(start, auto_idx, dir, s_v, s_h);
                s_cfg[0].1 = new_dir;
            }
        }

        if let Some((s_cfg, s_v, s_h)) = single {
            let single_cost = self.solution_cost(&s_cfg, &s_v, &s_h);
            let multi_cost = self.solution_cost(&multi.0, &multi.1, &multi.2);
            if single_cost <= multi_cost {
                self.output(&s_cfg, &s_v, &s_h);
            } else {
                self.output(&multi.0, &multi.1, &multi.2);
            }
        } else {
            self.output(&multi.0, &multi.1, &multi.2);
        }
    }
}

fn main() {
    let mut solver = Solver::new();
    solver.solve();
}
