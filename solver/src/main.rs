use proconio::input;

const MAX_N: usize = 20;
const K: usize = MAX_N * MAX_N;

struct Solver {
    n: usize,
    a_k: i64,
    a_m: i64,
    a_w: i64,
    wall_v: Vec<String>,
    wall_h: Vec<String>,
}

impl Solver {
    fn new() -> Self {
        input! {
            n: usize,
            a_k: i64,
            a_m: i64,
            a_w: i64,
            wall_v: [String; MAX_N],
            wall_h: [String; MAX_N -1],
        }

        Solver {
            n,
            a_k,
            a_m,
            a_w,
            wall_v,
            wall_h,
        }
    }

    fn solve(&mut self) {
        println!("{}", K);

        for i in 0..MAX_N {
            for j in 0..MAX_N {
                // 1 state, start at (i,j) facing Up, always turn right
                println!("1 {} {} U", i, j);
                println!("R 0 R 0");
            }
        }

        // No new walls
        for _ in 0..MAX_N {
            println!("{}", "0".repeat(MAX_N - 1));
        }
        for _ in 0..MAX_N - 1 {
            println!("{}", "0".repeat(MAX_N));
        }
    }
}

fn main() {
    let mut solver = Solver::new();
    solver.solve();
}
