use proconio::input;
use std::collections::VecDeque;

// --- Dinic's max flow (flat edge array for borrow-checker friendliness) ---
struct MaxFlow {
    graph: Vec<Vec<usize>>,
    to: Vec<usize>,
    cap: Vec<i64>,
    rev: Vec<usize>,
}

impl MaxFlow {
    fn new(n: usize) -> Self {
        MaxFlow {
            graph: vec![vec![]; n],
            to: vec![],
            cap: vec![],
            rev: vec![],
        }
    }

    fn add_edge(&mut self, from: usize, to: usize, cap: i64) {
        let m = self.to.len();
        self.graph[from].push(m);
        self.graph[to].push(m + 1);
        self.to.push(to);
        self.to.push(from);
        self.cap.push(cap);
        self.cap.push(0);
        self.rev.push(m + 1);
        self.rev.push(m);
    }

    fn bfs(&self, s: usize) -> Vec<i32> {
        let n = self.graph.len();
        let mut level = vec![-1i32; n];
        level[s] = 0;
        let mut q = VecDeque::new();
        q.push_back(s);
        while let Some(v) = q.pop_front() {
            for &e in &self.graph[v] {
                if self.cap[e] > 0 && level[self.to[e]] < 0 {
                    level[self.to[e]] = level[v] + 1;
                    q.push_back(self.to[e]);
                }
            }
        }
        level
    }

    fn dfs(&mut self, v: usize, t: usize, f: i64, level: &[i32], iter: &mut [usize]) -> i64 {
        if v == t {
            return f;
        }
        while iter[v] < self.graph[v].len() {
            let e = self.graph[v][iter[v]];
            let to = self.to[e];
            if self.cap[e] > 0 && level[v] < level[to] {
                let d = self.dfs(to, t, f.min(self.cap[e]), level, iter);
                if d > 0 {
                    self.cap[e] -= d;
                    self.cap[self.rev[e]] += d;
                    return d;
                }
            }
            iter[v] += 1;
        }
        0
    }

    fn max_flow(&mut self, s: usize, t: usize) -> i64 {
        let mut flow = 0;
        loop {
            let level = self.bfs(s);
            if level[t] < 0 {
                break;
            }
            let n = self.graph.len();
            let mut iter = vec![0usize; n];
            loop {
                let f = self.dfs(s, t, i64::MAX, &level, &mut iter);
                if f == 0 {
                    break;
                }
                flow += f;
            }
        }
        flow
    }

    /// After max_flow: find nodes reachable from s in the residual graph
    fn reachable_from(&self, s: usize) -> Vec<bool> {
        let n = self.graph.len();
        let mut visited = vec![false; n];
        visited[s] = true;
        let mut q = VecDeque::new();
        q.push_back(s);
        while let Some(v) = q.pop_front() {
            for &e in &self.graph[v] {
                if self.cap[e] > 0 && !visited[self.to[e]] {
                    visited[self.to[e]] = true;
                    q.push_back(self.to[e]);
                }
            }
        }
        visited
    }
}

fn main() {
    input! {
        n: usize,
        _a_k: i64,
        _a_m: i64,
        _a_w: i64,
        wall_v: [String; n],
        wall_h: [String; n - 1],
    }

    // Parse walls
    let v: Vec<Vec<bool>> = wall_v
        .iter()
        .map(|s| s.chars().map(|c| c == '1').collect())
        .collect();
    let h: Vec<Vec<bool>> = wall_h
        .iter()
        .map(|s| s.chars().map(|c| c == '1').collect())
        .collect();

    // Compute row segments: (row, start_col, end_col) inclusive
    let mut row_segs: Vec<(usize, usize, usize)> = Vec::new();
    let mut row_seg_id = vec![vec![0usize; n]; n];
    for i in 0..n {
        let mut j = 0;
        while j < n {
            let start = j;
            while j < n - 1 && !v[i][j] {
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

    // Compute column segments: (col, start_row, end_row) inclusive
    let mut col_segs: Vec<(usize, usize, usize)> = Vec::new();
    let mut col_seg_id = vec![vec![0usize; n]; n];
    for j in 0..n {
        let mut i = 0;
        while i < n {
            let start = i;
            while i < n - 1 && !h[i][j] {
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
    let seg_cost = |s: usize, e: usize| -> i64 { if s == e { 1 } else { 2 } };

    // --- Minimum weight vertex cover via max-flow ---
    // Bipartite graph: row segments (left) <-> column segments (right)
    // Edge for each cell connecting its row and column segment.
    // Source -> row_seg (cap=cost), col_seg -> sink (cap=cost), row_seg -> col_seg (cap=INF)
    // Min cut = min vertex cover weight.
    let source = 0;
    let sink = 1;
    let total_nodes = 2 + p + q;
    let mut flow = MaxFlow::new(total_nodes);

    for i in 0..p {
        let (_, s, e) = row_segs[i];
        flow.add_edge(source, 2 + i, seg_cost(s, e));
    }
    for j in 0..q {
        let (_, s, e) = col_segs[j];
        flow.add_edge(2 + p + j, sink, seg_cost(s, e));
    }
    for i in 0..n {
        for j in 0..n {
            let r = row_seg_id[i][j];
            let c = col_seg_id[i][j];
            flow.add_edge(2 + r, 2 + p + c, i64::MAX / 2);
        }
    }

    let _min_cost = flow.max_flow(source, sink);
    let reachable = flow.reachable_from(source);

    // Vertex cover: left (row) NOT reachable, right (col) reachable
    let mut selected_row = vec![false; p];
    let mut selected_col = vec![false; q];
    for i in 0..p {
        selected_row[i] = !reachable[2 + i];
    }
    for j in 0..q {
        selected_col[j] = reachable[2 + p + j];
    }

    // Output robots
    let num_robots: usize =
        selected_row.iter().filter(|&&x| x).count() + selected_col.iter().filter(|&&x| x).count();
    println!("{}", num_robots);

    // Row robots: 2-state U-turn (F 0 R 1 / R 0 R 0), start facing R
    for i in 0..p {
        if !selected_row[i] {
            continue;
        }
        let (row, sc, ec) = row_segs[i];
        if sc == ec {
            println!("1 {} {} R", row, sc);
            println!("R 0 R 0");
        } else {
            println!("2 {} {} R", row, sc);
            println!("F 0 R 1");
            println!("R 0 R 0");
        }
    }

    // Column robots: 2-state U-turn, start facing D
    for j in 0..q {
        if !selected_col[j] {
            continue;
        }
        let (col, sr, er) = col_segs[j];
        if sr == er {
            println!("1 {} {} D", sr, col);
            println!("R 0 R 0");
        } else {
            println!("2 {} {} D", sr, col);
            println!("F 0 R 1");
            println!("R 0 R 0");
        }
    }

    // No new walls
    for _ in 0..n {
        println!("{}", "0".repeat(n - 1));
    }
    for _ in 0..n - 1 {
        println!("{}", "0".repeat(n));
    }
}
