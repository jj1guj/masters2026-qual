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

/// All automaton definitions: (table slice, num_states)
struct AutomatonDef {
    table: &'static [(u8, usize, u8, usize)],
}

const AUTOMATA: [AutomatonDef; 8] = [
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

    /// 壁を縦横に伸ばし、長方形化する。追加した壁を (v_out, h_out) で返す。
    fn extend_walls(&mut self) -> ([[i32; MAX_N - 1]; MAX_N], [[i32; MAX_N]; MAX_N - 1]) {
        let mut v_out = [[0i32; MAX_N - 1]; MAX_N];
        let mut h_out = [[0i32; MAX_N]; MAX_N - 1];

        // 縦方向: 各列境界 j について壁を上下に伸ばす
        for j in 0..MAX_N - 1 {
            // 下方向に伸ばす
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
            // 上方向に伸ばす
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

        // 横方向: 各行境界 i について壁を左右に伸ばす
        for i in 0..MAX_N - 1 {
            // 右方向に伸ばす
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
            // 左方向に伸ばす
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

        // 長方形化: 隣接セルの壁パターンが異なる場合に壁を追加（収束まで繰り返す）
        self.rectify(&mut v_out, &mut h_out);

        (v_out, h_out)
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

    fn solve(&mut self) {
        let (mut v_out, mut h_out) = self.extend_walls();
        loop {
            self.try_merge_regions(&mut v_out, &mut h_out);
            if !self.try_merge_multi_regions(&mut v_out, &mut h_out) {
                break;
            }
        }
        let configs = self.find_robot_configs();
        self.output(&configs, &v_out, &h_out);
    }
}

fn main() {
    let mut solver = Solver::new();
    solver.solve();
}
