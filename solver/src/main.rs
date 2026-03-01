use proconio::{input, marker::Chars};

const MAX_N: usize = 20;
const K: usize = MAX_N * MAX_N;

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

    fn solve(&mut self) {
        // 壁を伸ばす
        let mut v_out = [[0; MAX_N - 1]; MAX_N];
        let mut h_out = [[0; MAX_N]; MAX_N - 1];

        let mut start_pos = 0;
        let mut end_pos = 0;
        let mut flg = false;
        // 縦方向: 各列境界 j について壁を上下に伸ばす
        for j in 0..MAX_N - 1 {
            // 下方向に伸ばす
            for i in 0..MAX_N {
                if self.v[i][j] || v_out[i][j] == 1 {
                    // i+1 以降に下向きに伸ばす
                    for r in (i + 1)..MAX_N {
                        // 横壁との衝突チェック: row r の上端交点
                        if r > 0 {
                            let hit = (j < MAX_N - 1 && self.h[r - 1][j + 1]) || self.h[r - 1][j];
                            if hit {
                                break;
                            }
                        }
                        if self.v[r][j] || v_out[r][j] == 1 {
                            break; // 既存の壁に到達
                        }
                        v_out[r][j] = 1;
                    }
                }
            }
            // 上方向に伸ばす
            for i in (0..MAX_N).rev() {
                if self.v[i][j] || v_out[i][j] == 1 {
                    // i-1 以上に上向きに伸ばす
                    for r in (0..i).rev() {
                        // 横壁との衝突チェック: row r の下端交点
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
                        // 縦壁との衝突チェック: col c の左端交点
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
                    }
                }
            }
            // 左方向に伸ばす
            for j in (0..MAX_N).rev() {
                if self.h[i][j] || h_out[i][j] == 1 {
                    for c in (0..j).rev() {
                        // 縦壁との衝突チェック: col c の右端交点
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
                    }
                }
            }
        }

        println!("{}", 1);

        println!("6 0 0 R");
        println!("F 0 R 1");
        println!("F 2 R 2");
        println!("R 3 R 3");
        println!("F 3 L 4");
        println!("F 5 L 5");
        println!("L 0 L 0");

        // No new walls
        for i in 0..MAX_N {
            for j in 0..MAX_N - 1 {
                print!("{}", v_out[i][j]);
            }
            println!("");
        }
        for i in 0..MAX_N - 1 {
            for j in 0..MAX_N {
                print!("{}", h_out[i][j]);
            }
            println!("");
        }
    }
}

fn main() {
    let mut solver = Solver::new();
    solver.solve();
}
