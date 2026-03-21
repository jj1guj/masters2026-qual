//! 焼き鈍しでオートマトンセットを探索するツール
//!
//! 使い方:
//!   cargo run --release --bin annealer -- [入力ディレクトリ] [制限秒数]
//!
//! 例:
//!   cargo run --release --bin annealer -- tools/inA 60
//!
//! 仕様:
//!   - 状態数 NUM_STATES のオートマトンを NUM_AUTOS 個用意する
//!   - 各テストケースにつき、全オートマトン × 全初期方向 でシミュレーション
//!   - 各テストケースの結果 = 最大カバーマス数
//!   - 合計スコアを目的関数として焼き鈍し

use std::fs;
use std::time::{Duration, Instant};

const N: usize = 20;
// 導入するオートマトンの個数は 4 のまま固定
const NUM_AUTOS: usize = 6;
// 各オートマトンが持つ内部状態数を 4 -> 5 に増やす
const NUM_STATES: usize = 6;

// State space size: (row, col, dir, auto_state) combinations
const STATE_SPACE: usize = N * N * 4 * NUM_STATES; // 8000

// Automaton entry: (action_no_wall, next_no_wall, action_wall, next_wall)
// action: 0=F(forward), 1=R(right), 2=L(left)
// wall 時の action は 1 か 2 のみ (前方が壁なら直進不可)
type AutoEntry = (u8, usize, u8, usize);
type AutoTable = [AutoEntry; NUM_STATES];
type AutoSet = [AutoTable; NUM_AUTOS];

#[derive(Clone, Copy)]
struct EvalStats {
    total: usize,
    full: usize,
    near_399: usize,
    near_398: usize,
    near_395: usize,
    near_380: usize,
    min_cover: usize,
}

#[derive(Clone, Copy)]
struct Elite {
    set: AutoSet,
    stats: EvalStats,
    combined: usize,
}

// --------- Board ---------

struct Board {
    v: [[bool; N - 1]; N], // v[i][j]: (i,j)-(i,j+1) 間の壁
    h: [[bool; N]; N - 1], // h[i][j]: (i,j)-(i+1,j) 間の壁
}

fn has_wall(board: &Board, row: usize, col: usize, dir: usize) -> bool {
    match dir {
        0 => row == 0 || board.h[row - 1][col], // U
        1 => col == N - 1 || board.v[row][col], // R
        2 => row == N - 1 || board.h[row][col], // D
        3 => col == 0 || board.v[row][col - 1], // L
        _ => unreachable!(),
    }
}

fn parse_board(content: &str) -> Board {
    let mut lines = content.lines();
    let _header = lines.next().expect("header line missing");

    let mut v = [[false; N - 1]; N];
    for i in 0..N {
        let line = lines.next().unwrap_or("").trim();
        for (j, c) in line.chars().enumerate() {
            if j < N - 1 && c == '1' {
                v[i][j] = true;
            }
        }
    }

    let mut h = [[false; N]; N - 1];
    for i in 0..N - 1 {
        let line = lines.next().unwrap_or("").trim();
        for (j, c) in line.chars().enumerate() {
            if j < N && c == '1' {
                h[i][j] = true;
            }
        }
    }

    Board { v, h }
}

// --------- Simulation ---------

/// 1回のシミュレーション。周期中に訪れたマス数を返す。
/// visited_at / touched は呼び出し間で使い回すバッファ。
fn simulate(
    board: &Board,
    auto_table: &AutoTable,
    start_row: usize,
    start_col: usize,
    start_dir: usize,
    visited_at: &mut [i32; STATE_SPACE],
    touched: &mut Vec<usize>,
    history: &mut Vec<u16>, // encoded as row*N+col
) -> usize {
    touched.clear();
    history.clear();

    let mut row = start_row;
    let mut col = start_col;
    let mut dir = start_dir;
    let mut state = 0usize;

    loop {
        let idx = ((row * N + col) * 4 + dir) * NUM_STATES + state;
        let at = visited_at[idx];

        if at >= 0 {
            // 周期始点が見つかった → visited_at をリセットして結果を返す
            for &i in touched.iter() {
                visited_at[i] = -1;
            }
            let cycle_start = at as usize;
            let mut cell_visited = [false; N * N];
            for i in cycle_start..history.len() {
                cell_visited[history[i] as usize] = true;
            }
            return cell_visited.iter().filter(|&&x| x).count();
        }

        visited_at[idx] = history.len() as i32;
        touched.push(idx);
        history.push((row * N + col) as u16);

        let wall = has_wall(board, row, col, dir);
        let (action, next_state) = if wall {
            (auto_table[state].2, auto_table[state].3)
        } else {
            (auto_table[state].0, auto_table[state].1)
        };

        match action {
            0 => match dir {
                // F: 前進
                0 => row -= 1,
                1 => col += 1,
                2 => row += 1,
                3 => col -= 1,
                _ => unreachable!(),
            },
            1 => dir = (dir + 1) % 4, // R: 右折
            2 => dir = (dir + 3) % 4, // L: 左折
            _ => unreachable!(),
        }
        state = next_state;
    }
}

/// 全テストケース評価を返す。
/// テストケースをスレッド分割して並列実行し、各ケースで最大カバー数を取る。
fn eval_all(boards: &[Board], auto_set: &AutoSet, start_row: usize, start_col: usize) -> EvalStats {
    let n_boards = boards.len();

    if n_boards == 0 {
        return EvalStats {
            total: 0,
            full: 0,
            near_399: 0,
            near_398: 0,
            near_395: 0,
            near_380: 0,
            min_cover: 0,
        };
    }

    let hw_threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    let env_threads = std::env::var("ANNEALER_THREADS")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .filter(|&x| x > 0);
    let num_threads = env_threads.unwrap_or(hw_threads).min(n_boards).max(1);
    let chunk_size = n_boards.div_ceil(num_threads);

    let partials = std::thread::scope(|scope| {
        let mut handles = Vec::new();
        for t in 0..num_threads {
            let l = t * chunk_size;
            if l >= n_boards {
                break;
            }
            let r = (l + chunk_size).min(n_boards);
            handles.push(scope.spawn(move || {
                let mut total = 0usize;
                let mut full_count = 0usize;
                let mut near_399 = 0usize;
                let mut near_398 = 0usize;
                let mut near_395 = 0usize;
                let mut near_380 = 0usize;
                let mut min_cover = N * N;

                let mut visited_at = [-1i32; STATE_SPACE];
                let mut touched = Vec::with_capacity(STATE_SPACE);
                let mut history = Vec::with_capacity(STATE_SPACE);

                for b in &boards[l..r] {
                    let mut best = 0usize;
                    for auto_table in auto_set {
                        for dir in 0..4 {
                            let cnt = simulate(
                                b,
                                auto_table,
                                start_row,
                                start_col,
                                dir,
                                &mut visited_at,
                                &mut touched,
                                &mut history,
                            );
                            if cnt > best {
                                best = cnt;
                            }
                        }
                    }

                    total += best;
                    if best < min_cover {
                        min_cover = best;
                    }
                    if best == N * N {
                        full_count += 1;
                    }
                    if best >= 399 {
                        near_399 += 1;
                    }
                    if best >= 398 {
                        near_398 += 1;
                    }
                    if best >= 395 {
                        near_395 += 1;
                    }
                    if best >= 380 {
                        near_380 += 1;
                    }
                }

                (
                    total, full_count, near_399, near_398, near_395, near_380, min_cover,
                )
            }));
        }
        handles
            .into_iter()
            .map(|h| h.join().unwrap())
            .collect::<Vec<_>>()
    });

    let mut total = 0usize;
    let mut full_count = 0usize;
    let mut near_399 = 0usize;
    let mut near_398 = 0usize;
    let mut near_395 = 0usize;
    let mut near_380 = 0usize;
    let mut min_cover = N * N;
    for (t, f, n399, n398, n395, n380, mn) in partials {
        total += t;
        full_count += f;
        near_399 += n399;
        near_398 += n398;
        near_395 += n395;
        near_380 += n380;
        if mn < min_cover {
            min_cover = mn;
        }
    }

    EvalStats {
        total,
        full: full_count,
        near_399,
        near_398,
        near_395,
        near_380,
        min_cover,
    }
}

// --------- PRNG ---------

fn xorshift64(seed: &mut u64) -> u64 {
    *seed ^= *seed << 7;
    *seed ^= *seed >> 9;
    *seed
}

fn rand_range(seed: &mut u64, n: usize) -> usize {
    (xorshift64(seed) as usize) % n
}

fn rand_f64(seed: &mut u64) -> f64 {
    (xorshift64(seed) as f64) / (u64::MAX as f64)
}

// --------- Automata manipulation ---------

/// state 0 から全状態が到達可能かどうかを判定する。
/// 遷移グラフを BFS で探索し、NUM_STATES 個全てに到達できれば true。
fn is_auto_table_valid(table: &AutoTable) -> bool {
    let mut reachable = [false; NUM_STATES];
    reachable[0] = true;
    let mut queue = [0usize; NUM_STATES];
    let mut head = 0;
    let mut tail = 1;
    queue[0] = 0;
    while head < tail {
        let s = queue[head];
        head += 1;
        for &next in &[table[s].1, table[s].3] {
            if !reachable[next] {
                reachable[next] = true;
                queue[tail] = next;
                tail += 1;
            }
        }
    }
    reachable.iter().all(|&r| r)
}

fn is_auto_set_valid(auto_set: &AutoSet) -> bool {
    auto_set.iter().all(is_auto_table_valid)
}

fn random_auto_set(seed: &mut u64) -> AutoSet {
    loop {
        let mut result = [[(0u8, 0usize, 1u8, 0usize); NUM_STATES]; NUM_AUTOS];
        for k in 0..NUM_AUTOS {
            for s in 0..NUM_STATES {
                let a0 = rand_range(seed, 3) as u8; // 0,1,2
                let b0 = rand_range(seed, NUM_STATES);
                let a1 = (rand_range(seed, 2) + 1) as u8; // 1 or 2
                let b1 = rand_range(seed, NUM_STATES);
                result[k][s] = (a0, b0, a1, b1);
            }
        }
        if is_auto_set_valid(&result) {
            return result;
        }
    }
}

/// オートマトンセットをランダムに1か所変異させる。
/// 変異後も全オートマトンで全状態が到達可能である変異のみを返す。
fn mutate(auto_set: &AutoSet, seed: &mut u64) -> AutoSet {
    for _ in 0..1000 {
        let mut new_set = *auto_set;
        let mode = rand_range(seed, 100);

        if mode < 60 {
            // 小変異: 1 フィールドのみ変更
            let k = rand_range(seed, NUM_AUTOS);
            let s = rand_range(seed, NUM_STATES);
            let field = rand_range(seed, 4);
            match field {
                0 => new_set[k][s].0 = rand_range(seed, 3) as u8,
                1 => new_set[k][s].1 = rand_range(seed, NUM_STATES),
                2 => new_set[k][s].2 = (rand_range(seed, 2) + 1) as u8,
                3 => new_set[k][s].3 = rand_range(seed, NUM_STATES),
                _ => unreachable!(),
            }
        } else if mode < 85 {
            // 中変異: 1 状態行を丸ごと再生成
            let k = rand_range(seed, NUM_AUTOS);
            let s = rand_range(seed, NUM_STATES);
            new_set[k][s] = (
                rand_range(seed, 3) as u8,
                rand_range(seed, NUM_STATES),
                (rand_range(seed, 2) + 1) as u8,
                rand_range(seed, NUM_STATES),
            );
        } else if mode < 97 {
            // 大変異: 1 オートマトンを再生成
            let k = rand_range(seed, NUM_AUTOS);
            for s in 0..NUM_STATES {
                new_set[k][s] = (
                    rand_range(seed, 3) as u8,
                    rand_range(seed, NUM_STATES),
                    (rand_range(seed, 2) + 1) as u8,
                    rand_range(seed, NUM_STATES),
                );
            }
        } else {
            // まれに全再生成（リスタート相当）
            new_set = random_auto_set(seed);
        }

        if is_auto_set_valid(&new_set) {
            return new_set;
        }
    }
    // 1000回試してもなければ元のまま返す（実質ほぼ起こらない）
    *auto_set
}

fn mutate_many(auto_set: &AutoSet, seed: &mut u64, steps: usize) -> AutoSet {
    let mut cur = *auto_set;
    for _ in 0..steps {
        cur = mutate(&cur, seed);
    }
    cur
}

fn random_entry(seed: &mut u64) -> AutoEntry {
    (
        rand_range(seed, 3) as u8,
        rand_range(seed, NUM_STATES),
        (rand_range(seed, 2) + 1) as u8,
        rand_range(seed, NUM_STATES),
    )
}

fn all_possible_entries() -> Vec<AutoEntry> {
    let mut out = Vec::with_capacity(3 * NUM_STATES * 2 * NUM_STATES);
    for a0 in 0..3u8 {
        for b0 in 0..NUM_STATES {
            for a1 in 1..=2u8 {
                for b1 in 0..NUM_STATES {
                    out.push((a0, b0, a1, b1));
                }
            }
        }
    }
    out
}

fn build_row_pool(elites: &[Elite], seed_sets: &[AutoSet]) -> Vec<AutoEntry> {
    let mut pool: Vec<AutoEntry> = Vec::new();
    for e in elites {
        for k in 0..NUM_AUTOS {
            for s in 0..NUM_STATES {
                pool.push(e.set[k][s]);
            }
        }
    }
    for set in seed_sets {
        for k in 0..NUM_AUTOS {
            for s in 0..NUM_STATES {
                pool.push(set[k][s]);
            }
        }
    }
    pool.sort_unstable();
    pool.dedup();
    pool
}

fn build_slot_pool(elites: &[Elite], seed_sets: &[AutoSet]) -> Vec<Vec<Vec<AutoEntry>>> {
    let mut pool = vec![vec![Vec::<AutoEntry>::new(); NUM_STATES]; NUM_AUTOS];
    for e in elites {
        for k in 0..NUM_AUTOS {
            for s in 0..NUM_STATES {
                pool[k][s].push(e.set[k][s]);
            }
        }
    }
    for set in seed_sets {
        for k in 0..NUM_AUTOS {
            for s in 0..NUM_STATES {
                pool[k][s].push(set[k][s]);
            }
        }
    }
    for k in 0..NUM_AUTOS {
        for s in 0..NUM_STATES {
            pool[k][s].sort_unstable();
            pool[k][s].dedup();
        }
    }
    pool
}

fn update_elites(elites: &mut Vec<Elite>, cand: Elite) {
    if let Some(pos) = elites
        .iter()
        .position(|e| e.combined == cand.combined && e.stats.total == cand.stats.total)
    {
        if cand.stats.full > elites[pos].stats.full {
            elites[pos] = cand;
        }
        return;
    }
    elites.push(cand);
    elites.sort_by(|a, b| b.combined.cmp(&a.combined));
    if elites.len() > 8 {
        elites.truncate(8);
    }
}

fn crossover_auto_set(a: &AutoSet, b: &AutoSet, seed: &mut u64) -> AutoSet {
    // オートマトン単位 + 状態行単位の二段交叉
    let mut out = *a;
    for k in 0..NUM_AUTOS {
        if rand_range(seed, 2) == 1 {
            out[k] = b[k];
        }
        for s in 0..NUM_STATES {
            if rand_range(seed, 2) == 1 {
                out[k][s] = b[k][s];
            }
        }
    }
    if is_auto_set_valid(&out) {
        out
    } else {
        // 到達性を壊した場合は親を返す
        if rand_range(seed, 2) == 0 { *a } else { *b }
    }
}

fn predefined_seed_sets() -> Vec<AutoSet> {
    Vec::new()
}

// --------- Output ---------

fn action_char(a: u8) -> char {
    match a {
        0 => 'F',
        1 => 'R',
        2 => 'L',
        _ => unreachable!(),
    }
}

fn best_dir_for_automaton(
    auto_table: &AutoTable,
    boards: &[Board],
    start_row: usize,
    start_col: usize,
) -> usize {
    let mut best_dir = 0usize;
    let mut best_sum = 0usize;

    let mut visited_at = [-1i32; STATE_SPACE];
    let mut touched = Vec::with_capacity(STATE_SPACE);
    let mut history = Vec::with_capacity(STATE_SPACE);

    for dir in 0..4 {
        let mut sum = 0usize;
        for b in boards {
            sum += simulate(
                b,
                auto_table,
                start_row,
                start_col,
                dir,
                &mut visited_at,
                &mut touched,
                &mut history,
            );
        }
        if sum > best_sum {
            best_sum = sum;
            best_dir = dir;
        }
    }
    best_dir
}

/// Visualizer にそのまま貼れる解フォーマットを stdout に出力する。
/// 各オートマトンについて 1 体ずつ（合計 NUM_AUTOS 体）中央配置する。
/// 各オートマトンの向きは、入力集合に対する合計カバー数が最大となる方向を採用する。
fn print_solution_output(auto_set: &AutoSet, boards: &[Board], start_row: usize, start_col: usize) {
    let dirs = ['U', 'R', 'D', 'L'];
    let center_r = N / 2 - 1;
    let center_c = N / 2 - 1;

    let k = NUM_AUTOS;
    println!("{}", k);

    for auto_table in auto_set.iter() {
        let best_dir = best_dir_for_automaton(auto_table, boards, start_row, start_col);

        println!(
            "{} {} {} {}",
            NUM_STATES, center_r, center_c, dirs[best_dir]
        );
        for &(a0, b0, a1, b1) in auto_table.iter() {
            println!("{} {} {} {}", action_char(a0), b0, action_char(a1), b1);
        }
    }

    // 追加壁は置かない
    for _ in 0..N {
        println!("{}", "0".repeat(N - 1));
    }
    for _ in 0..(N - 1) {
        println!("{}", "0".repeat(N));
    }
}

/// デバッグ・共有用の可読形式を stderr に出力する。
fn print_auto_set_readable(
    auto_set: &AutoSet,
    boards: &[Board],
    start_row: usize,
    start_col: usize,
) {
    let action_name = |a: u8| match a {
        0 => "F",
        1 => "R",
        2 => "L",
        _ => "?",
    };

    eprintln!("// ---- Best AutoSet (Rust 定数形式) ----");
    for (k, auto_table) in auto_set.iter().enumerate() {
        eprint!("const AUTO_{k}: [(u8, usize, u8, usize); {NUM_STATES}] = [");
        for (s, &(a0, b0, a1, b1)) in auto_table.iter().enumerate() {
            if s > 0 {
                eprint!(", ");
            }
            eprint!("({}, {b0}, {}, {b1})", a0, a1);
        }
        eprintln!("];");
    }

    eprintln!();
    eprintln!("// ---- 人間可読形式 ----");
    for (k, auto_table) in auto_set.iter().enumerate() {
        eprintln!("// Automaton {k}:");
        for (s, &(a0, b0, a1, b1)) in auto_table.iter().enumerate() {
            eprintln!(
                "//   state {s}: no_wall=({}, next={b0})  wall=({}, next={b1})",
                action_name(a0),
                action_name(a1),
            );
        }
    }

    // コメントなしのコピペ専用（各オートマトン1ブロックずつ）
    eprintln!();
    eprintln!("AUTOMATA_COPYABLE_BEGIN");
    let dirs = ['U', 'R', 'D', 'L'];
    let center_r = N / 2 - 1;
    let center_c = N / 2 - 1;
    for (k, auto_table) in auto_set.iter().enumerate() {
        eprintln!("PATTERN {}", k);
        eprintln!("1");
        let best_dir = best_dir_for_automaton(auto_table, boards, start_row, start_col);
        eprintln!(
            "{} {} {} {}",
            NUM_STATES, center_r, center_c, dirs[best_dir]
        );
        for &(a0, b0, a1, b1) in auto_table.iter() {
            eprintln!("{} {} {} {}", action_char(a0), b0, action_char(a1), b1);
        }
        if k + 1 < auto_set.len() {
            eprintln!();
        }
    }
    eprintln!("AUTOMATA_COPYABLE_END");
}

// --------- Main ---------

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let input_dir = args.get(1).map(String::as_str).unwrap_or("tools/inB");
    let time_secs: u64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(60);
    let mut seed: u64 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or_else(|| {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_else(|_| Duration::from_secs(0));
        now.as_nanos() as u64 ^ 0x1234567890ABCDEFu64
    });

    // テストケース読み込み
    let mut entries: Vec<_> = fs::read_dir(input_dir)
        .unwrap_or_else(|e| panic!("ディレクトリを開けません: {input_dir} ({e})"))
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "txt"))
        .collect();
    entries.sort_by_key(|e| e.file_name());

    let boards: Vec<Board> = entries
        .iter()
        .map(|e| {
            let content = fs::read_to_string(e.path()).expect("ファイル読み込み失敗");
            parse_board(&content)
        })
        .collect();

    eprintln!(
        "テストケース {}件 を {} から読み込み完了",
        boards.len(),
        input_dir
    );

    let max_cells = boards.len() * N * N;
    let long_run = time_secs >= 1800;

    // 開始位置: 盤面の中央 (9, 9)
    let start_row = N / 2 - 1; // 9
    let start_col = N / 2 - 1; // 9

    eprintln!("乱数seed: {}", seed);

    // 厳格なベスト比較用スコア（lexicographicに近い）
    let combined = |st: EvalStats| -> usize {
        st.full * 1_000_000_000_000
            + st.near_399 * 1_000_000_000
            + st.near_398 * 10_000_000
            + st.near_395 * 100_000
            + st.near_380 * 5_000
            + st.min_cover * 200
            + st.total
    };

    // 焼き鈍し受理用スコア（時間経過で重みを変える）
    let anneal_score = |st: EvalStats, full_w: i64, near_w: i64| -> i64 {
        (st.full as i64) * full_w + (st.near_399 as i64) * near_w + st.total as i64
    };

    // 初期解: 既知良解 + ランダム複数個から選ぶ
    let mut initial_sets = predefined_seed_sets();
    let extra_initial = if long_run { 24 } else { 8 };
    for _ in 0..extra_initial {
        initial_sets.push(random_auto_set(&mut seed));
    }

    let mut elites: Vec<Elite> = Vec::new();
    let mut best_set = initial_sets[0];
    let mut best_stats = eval_all(&boards, &best_set, start_row, start_col);
    let mut best_combined = combined(best_stats);

    for set in initial_sets {
        let st = eval_all(&boards, &set, start_row, start_col);
        let cmb = combined(st);
        let e = Elite {
            set,
            stats: st,
            combined: cmb,
        };
        update_elites(&mut elites, e);
        if cmb > best_combined {
            best_combined = cmb;
            best_stats = st;
            best_set = set;
        }
    }

    let mut current_set = best_set;
    let mut current_stats = best_stats;
    let mut current_anneal = anneal_score(current_stats, 60_000, 800);

    eprintln!(
        "初期スコア: {} / {} ({:.1}%)  全カバー: {}/{}  399+:{} 398+:{} 395+:{} 380+:{} min:{} combined={}",
        best_stats.total,
        max_cells,
        best_stats.total as f64 / max_cells as f64 * 100.0,
        best_stats.full,
        boards.len(),
        best_stats.near_399,
        best_stats.near_398,
        best_stats.near_395,
        best_stats.near_380,
        best_stats.min_cover,
        best_combined,
    );

    let start = Instant::now();
    let total_limit = Duration::from_secs(time_secs);
    let hard_deadline = start + total_limit;
    let local_phase_reserve = if long_run {
        Duration::from_secs(600)
    } else {
        Duration::from_secs_f64((time_secs as f64 * (1.0 - 0.82)).max(1.0))
    };
    let anneal_deadline = hard_deadline
        .checked_sub(local_phase_reserve)
        .unwrap_or(start + Duration::from_secs(1));

    // 焼き鈍しのパラメータ
    // combined_score の単位に合わせた温度スケール:
    //   1ケース全カバー = N*N+1 = 401 点 なので
    //   t_start ≈ 1ケース全カバー相当 (400), t_end = 1.0
    let t_start = if long_run { 3_000_000.0 } else { 20_000_000.0 };
    let t_end = 1.0f64;

    let mut iter = 0u64;
    let mut accepted = 0u64;
    let mut last_print = start;
    let mut last_improve_at = start;
    let log_interval = if long_run {
        Duration::from_secs(30)
    } else {
        Duration::from_secs(5)
    };

    while Instant::now() < anneal_deadline {
        let elapsed = start.elapsed().as_secs_f64();
        let progress = (elapsed / total_limit.as_secs_f64()).min(1.0);
        let temp = t_start * (t_end / t_start).powf(progress);
        let (full_w, near_w) = if progress < 0.35 {
            (60_000_i64, 800_i64)
        } else if progress < 0.75 {
            (140_000_i64, 1_500_i64)
        } else {
            (260_000_i64, 3_000_i64)
        };

        // 1反復で複数候補を評価し、最良候補だけを焼き鈍し判定にかける
        let trials = if progress < 0.5 {
            2usize
        } else if progress < 0.85 {
            3usize
        } else {
            4usize
        };

        let mut cand_best_set = current_set;
        let mut cand_best_stats = current_stats;
        let mut cand_best_anneal = i64::MIN;

        for _ in 0..trials {
            let r = rand_range(&mut seed, 100);
            let (base_set, steps) = if r < 45 {
                (current_set, 1 + rand_range(&mut seed, 4))
            } else if r < 80 {
                let idx = rand_range(&mut seed, elites.len());
                (elites[idx].set, 3 + rand_range(&mut seed, 10))
            } else if r < 92 {
                (best_set, 1 + rand_range(&mut seed, 6))
            } else if r < 99 && elites.len() >= 2 {
                let i = rand_range(&mut seed, elites.len());
                let mut j = rand_range(&mut seed, elites.len());
                if j == i {
                    j = (j + 1) % elites.len();
                }
                let cross = crossover_auto_set(&elites[i].set, &elites[j].set, &mut seed);
                (cross, 2 + rand_range(&mut seed, 6))
            } else {
                // 低確率でランダム新規も混ぜる
                (random_auto_set(&mut seed), 1 + rand_range(&mut seed, 2))
            };

            let new_set = mutate_many(&base_set, &mut seed, steps);
            let new_stats = eval_all(&boards, &new_set, start_row, start_col);
            let new_anneal = anneal_score(new_stats, full_w, near_w);
            if new_anneal > cand_best_anneal {
                cand_best_set = new_set;
                cand_best_stats = new_stats;
                cand_best_anneal = new_anneal;
            }
        }

        let new_set = cand_best_set;
        let new_stats = cand_best_stats;
        let new_combined = combined(new_stats);

        let delta = (cand_best_anneal - current_anneal) as f64;
        let accept = if delta >= 0.0 {
            true
        } else {
            rand_f64(&mut seed) < (delta / temp).exp()
        };

        if accept {
            current_set = new_set;
            current_anneal = cand_best_anneal;
            current_stats = new_stats;
            accepted += 1;

            update_elites(
                &mut elites,
                Elite {
                    set: current_set,
                    stats: current_stats,
                    combined: new_combined,
                },
            );

            if new_combined > best_combined {
                best_combined = new_combined;
                best_stats = new_stats;
                best_set = current_set;
                last_improve_at = Instant::now();
                eprintln!(
                    "★ iter={iter}, 新ベスト: {} ({:.1}%)  全カバー: {}/{}  399+:{} 398+:{} 395+:{} 380+:{} min:{} combined={} T={temp:.1}",
                    best_stats.total,
                    best_stats.total as f64 / max_cells as f64 * 100.0,
                    best_stats.full,
                    boards.len(),
                    best_stats.near_399,
                    best_stats.near_398,
                    best_stats.near_395,
                    best_stats.near_380,
                    best_stats.min_cover,
                    best_combined,
                );
            }
        }

        iter += 1;

        // 改善が止まったら再始動（時間ベース）
        let stagnation_secs = if long_run {
            if best_stats.full >= 50 {
                480
            } else if best_stats.full >= 40 {
                120
            } else {
                90
            }
        } else if best_stats.full >= 50 {
            30
        } else if best_stats.full >= 40 {
            15
        } else {
            10
        };

        if last_improve_at.elapsed() >= Duration::from_secs(stagnation_secs) {
            let base = if best_stats.full >= 50 {
                if elites.len() >= 2 {
                    let i = rand_range(&mut seed, elites.len());
                    let mut j = rand_range(&mut seed, elites.len());
                    if j == i {
                        j = (j + 1) % elites.len();
                    }
                    crossover_auto_set(&elites[i].set, &elites[j].set, &mut seed)
                } else {
                    best_set
                }
            } else if rand_range(&mut seed, 100) < 45 {
                random_auto_set(&mut seed)
            } else if elites.len() >= 2 {
                let i = rand_range(&mut seed, elites.len());
                let mut j = rand_range(&mut seed, elites.len());
                if j == i {
                    j = (j + 1) % elites.len();
                }
                crossover_auto_set(&elites[i].set, &elites[j].set, &mut seed)
            } else {
                elites[0].set
            };
            let jump = if best_stats.full >= 50 {
                4 + rand_range(&mut seed, 9)
            } else {
                12 + rand_range(&mut seed, 22)
            };
            current_set = mutate_many(&base, &mut seed, jump);
            current_stats = eval_all(&boards, &current_set, start_row, start_col);
            let elapsed = start.elapsed().as_secs_f64();
            let progress = (elapsed / total_limit.as_secs_f64()).min(1.0);
            let (full_w, near_w) = if progress < 0.35 {
                (60_000_i64, 800_i64)
            } else if progress < 0.75 {
                (140_000_i64, 1_500_i64)
            } else {
                (260_000_i64, 3_000_i64)
            };
            current_anneal = anneal_score(current_stats, full_w, near_w);
            last_improve_at = Instant::now();
            eprintln!(
                "  iter={iter}, 停滞再始動(limit={}s): full={}/{}, 399+:{} 398+:{} 395+:{} 380+:{} min={} total={}",
                stagnation_secs,
                current_stats.full,
                boards.len(),
                current_stats.near_399,
                current_stats.near_398,
                current_stats.near_395,
                current_stats.near_380,
                current_stats.min_cover,
                current_stats.total,
            );
        }

        if last_print.elapsed() >= log_interval {
            eprintln!(
                "  iter={iter}, best_full={}/{}, best_total={}, best_min={}, current_full={}, T={temp:.2}, accept_rate={:.1}%",
                best_stats.full,
                boards.len(),
                best_stats.total,
                best_stats.min_cover,
                current_stats.full,
                accepted as f64 / iter as f64 * 100.0
            );
            last_print = Instant::now();
        }
    }

    // 後半は全カバー数を最優先した局所改善フェーズ
    let seed_sets = predefined_seed_sets();
    let mut row_pool = build_row_pool(&elites, &seed_sets);
    row_pool.extend(all_possible_entries());
    row_pool.sort_unstable();
    row_pool.dedup();
    let slot_pool = build_slot_pool(&elites, &seed_sets);
    let all_entries = all_possible_entries();
    let mut local_iter = 0u64;
    eprintln!(
        "  局所改善開始: 残り{}秒でfull重視探索 (pool_rows={})",
        hard_deadline
            .saturating_duration_since(Instant::now())
            .as_secs(),
        row_pool.len(),
    );

    // まずは同一スロット(k,s)の候補を総当たりで置換する座標改善を数パス実施
    let mut pass = 0usize;
    while Instant::now() < hard_deadline && pass < 5 {
        let mut improved = false;
        for k in 0..NUM_AUTOS {
            for s in 0..NUM_STATES {
                if Instant::now() >= hard_deadline {
                    break;
                }
                let cur = best_set[k][s];
                let mut row_best = cur;
                let mut row_best_stats = best_stats;
                let mut row_best_combined = best_combined;

                for &entry in all_entries.iter().chain(slot_pool[k][s].iter()) {
                    if entry == cur {
                        continue;
                    }
                    let mut cand = best_set;
                    cand[k][s] = entry;
                    if !is_auto_set_valid(&cand) {
                        continue;
                    }
                    let st = eval_all(&boards, &cand, start_row, start_col);
                    let cmb = combined(st);
                    if st.full > row_best_stats.full
                        || (st.full == row_best_stats.full && cmb > row_best_combined)
                    {
                        row_best = entry;
                        row_best_stats = st;
                        row_best_combined = cmb;
                    }
                }

                if row_best != cur {
                    best_set[k][s] = row_best;
                    best_stats = row_best_stats;
                    best_combined = row_best_combined;
                    improved = true;
                    eprintln!(
                        "◆ slot-pass{}: full={}/{} total={} min={}",
                        pass,
                        best_stats.full,
                        boards.len(),
                        best_stats.total,
                        best_stats.min_cover,
                    );
                }
            }
        }
        if !improved {
            break;
        }
        pass += 1;
    }

    while Instant::now() < hard_deadline {
        let remain = hard_deadline.saturating_duration_since(Instant::now());
        let final_full_mode = long_run && remain <= Duration::from_secs(600);

        if final_full_mode && local_iter == 0 {
            eprintln!("  終盤full専用モード開始: 残り10分");
        }

        // 一定間隔で、行単位の集中改善（best近傍）を実施
        let refine_period = if final_full_mode { 80 } else { 200 };
        if local_iter % refine_period == 0 {
            for k in 0..NUM_AUTOS {
                for s in 0..NUM_STATES {
                    let old = best_set[k][s];
                    let refine_trials = if final_full_mode { 40 } else { 18 };
                    for _ in 0..refine_trials {
                        let idx = rand_range(&mut seed, row_pool.len());
                        let repl = row_pool[idx];
                        if repl == old {
                            continue;
                        }
                        let mut cand = best_set;
                        cand[k][s] = repl;
                        if !is_auto_set_valid(&cand) {
                            continue;
                        }
                        let st = eval_all(&boards, &cand, start_row, start_col);
                        let cmb = combined(st);
                        let better = if final_full_mode {
                            st.full > best_stats.full
                                || (st.full == best_stats.full
                                    && (st.near_399 > best_stats.near_399
                                        || (st.near_399 == best_stats.near_399
                                            && st.total > best_stats.total)))
                        } else {
                            st.full > best_stats.full
                                || (st.full == best_stats.full && cmb > best_combined)
                        };

                        if better {
                            best_set = cand;
                            best_stats = st;
                            best_combined = cmb;
                            eprintln!(
                                "◆ row-refine: full={}/{} total={} min={}",
                                best_stats.full,
                                boards.len(),
                                best_stats.total,
                                best_stats.min_cover,
                            );
                        }
                    }
                }
            }
        }

        let mut cand = best_set;

        // 1回の提案で 1〜3 行を置換（row pool 由来中心）
        let changes = if final_full_mode {
            1 + rand_range(&mut seed, 2)
        } else {
            1 + rand_range(&mut seed, 3)
        };
        for _ in 0..changes {
            let k = rand_range(&mut seed, NUM_AUTOS);
            let s = rand_range(&mut seed, NUM_STATES);
            let use_pool = if final_full_mode {
                true
            } else {
                rand_range(&mut seed, 100) < 88
            };
            if use_pool {
                let idx = rand_range(&mut seed, row_pool.len());
                cand[k][s] = row_pool[idx];
            } else {
                cand[k][s] = random_entry(&mut seed);
            }
        }

        if !is_auto_set_valid(&cand) {
            local_iter += 1;
            continue;
        }

        let st = eval_all(&boards, &cand, start_row, start_col);
        let cmb = combined(st);

        // full最優先、終盤はより強くfull寄りの比較
        let better = if final_full_mode {
            st.full > best_stats.full
                || (st.full == best_stats.full
                    && (st.near_399 > best_stats.near_399
                        || (st.near_399 == best_stats.near_399 && st.total > best_stats.total)))
        } else {
            st.full > best_stats.full || (st.full == best_stats.full && cmb > best_combined)
        };

        if better {
            best_set = cand;
            best_stats = st;
            best_combined = cmb;
            eprintln!(
                "☆ local={local_iter}, 改善: full={}/{} total={} min={} 399+={} 398+={}",
                best_stats.full,
                boards.len(),
                best_stats.total,
                best_stats.min_cover,
                best_stats.near_399,
                best_stats.near_398,
            );
        }

        local_iter += 1;
    }

    eprintln!(
        "\n完了: 試行={iter}, ベストスコア={} / {} ({:.1}%)  全カバー: {}/{}  399+:{} 398+:{} 395+:{} 380+:{} min={}",
        best_stats.total,
        max_cells,
        best_stats.total as f64 / max_cells as f64 * 100.0,
        best_stats.full,
        boards.len(),
        best_stats.near_399,
        best_stats.near_398,
        best_stats.near_395,
        best_stats.near_380,
        best_stats.min_cover
    );
    eprintln!(
        "  ケースあたり平均: {:.1} / {} マス",
        best_stats.total as f64 / boards.len() as f64,
        N * N
    );

    // stdout: 貼り付け用の正規解フォーマット
    // stderr: 可読なオートマトン表示
    print_solution_output(&best_set, &boards, start_row, start_col);
    print_auto_set_readable(&best_set, &boards, start_row, start_col);
}
