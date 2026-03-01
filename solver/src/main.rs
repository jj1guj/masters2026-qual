use ac_library::MfGraph;
use proconio::input;

const N: usize = 20;

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

    fn solve(&self) {
        let n = self.n;

        // Compute row segments: (row, start_col, end_col) inclusive
        let mut row_segs: Vec<(usize, usize, usize)> = Vec::new();
        let mut row_seg_id = vec![vec![0usize; n]; n];
        for i in 0..n {
            let mut j = 0;
            while j < n {
                let start = j;
                while j < n - 1 && !self.v[i][j] {
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
                while i < n - 1 && !self.h[i][j] {
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
        for i in 0..n {
            for j in 0..n {
                let r = row_seg_id[i][j];
                let c = col_seg_id[i][j];
                graph.add_edge(2 + r, 2 + p + c, i64::MAX / 2);
            }
        }

        let _min_cost = graph.flow(source, sink);
        let reachable = graph.min_cut(source);

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
        let num_robots: usize = selected_row.iter().filter(|&&x| x).count()
            + selected_col.iter().filter(|&&x| x).count();
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
}

fn main() {
    let solver = Solver::new();
    solver.solve();
}
