use proconio::{input, marker::Chars};

const MAX_N: usize = 20;
const K: usize = MAX_N * MAX_N;
const DX: [i32; 4] = [-1, 0, 0, 1];
const DY: [i32; 4] = [0, -1, 1, 0];

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

    fn solve(&mut self) {
        // 壁を伸ばす
        let mut v_out = [[0; MAX_N - 1]; MAX_N];
        let mut h_out = [[0; MAX_N]; MAX_N - 1];

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
                        self.v[r][j] = true;
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
                        self.h[i][c] = true;
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
                        self.h[i][c] = true;
                    }
                }
            }
        }

        // 長方形化: 隣接セルの壁パターンが異なる場合に壁を追加
        // 収束するまで繰り返す
        loop {
            let mut changed = false;

            // 横方向チェック: (i,j) と (i,j+1) が壁なしで隣接している場合
            // 上下の壁パターンが異なれば縦壁を追加
            for i in 0..MAX_N {
                for j in 0..MAX_N - 1 {
                    if self.v[i][j] {
                        continue; // 既に壁がある
                    }
                    let mut need_wall = false;
                    // 上の壁チェック: h[i-1][j] vs h[i-1][j+1]
                    if i > 0 {
                        if self.h[i - 1][j] != self.h[i - 1][j + 1] {
                            need_wall = true;
                        }
                    }
                    // 下の壁チェック: h[i][j] vs h[i][j+1]
                    if i < MAX_N - 1 {
                        if self.h[i][j] != self.h[i][j + 1] {
                            need_wall = true;
                        }
                    }
                    if need_wall {
                        self.v[i][j] = true;
                        v_out[i][j] = 1;
                        changed = true;
                    }
                }
            }

            // 縦方向チェック: (i,j) と (i+1,j) が壁なしで隣接している場合
            // 左右の壁パターンが異なれば横壁を追加
            for i in 0..MAX_N - 1 {
                for j in 0..MAX_N {
                    if self.h[i][j] {
                        continue; // 既に壁がある
                    }
                    let mut need_wall = false;
                    // 左の壁チェック: v[i][j-1] vs v[i+1][j-1]
                    if j > 0 {
                        if self.v[i][j - 1] != self.v[i + 1][j - 1] {
                            need_wall = true;
                        }
                    }
                    // 右の壁チェック: v[i][j] vs v[i+1][j]
                    if j < MAX_N - 1 {
                        if self.v[i][j] != self.v[i + 1][j] {
                            need_wall = true;
                        }
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

        let mut area_map = [[-1; MAX_N]; MAX_N];
        let mut rect_count = 0;
        let mut robot_pos: Vec<(usize, usize)> = Vec::new();
        for i in 0..MAX_N {
            for j in 0..MAX_N {
                if area_map[i][j] == -1 {
                    rect_count += 1;
                    area_map[i][j] = rect_count;
                    robot_pos.push((i, j));
                    self.dfs((i, j), &mut area_map, &self.v, &self.h);
                }
            }
        }

        println!("{}", robot_pos.len());
        for pos in robot_pos {
            println!("6 {} {} R", pos.0, pos.1);
            println!("F 0 R 1");
            println!("F 2 R 2");
            println!("R 3 R 3");
            println!("F 3 L 4");
            println!("F 5 L 5");
            println!("L 0 L 0");
        }

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
