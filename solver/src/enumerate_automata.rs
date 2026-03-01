use std::collections::HashMap;
use std::io::{self, BufRead};

const N: usize = 20;
const DI: [i32; 4] = [-1, 0, 1, 0];
const DJ: [i32; 4] = [0, 1, 0, -1];

fn turn_right(d: usize) -> usize {
    (d + 1) % 4
}
fn turn_left(d: usize) -> usize {
    (d + 3) % 4
}

fn has_wall_ahead(v: &[Vec<bool>], h: &[Vec<bool>], r: usize, c: usize, d: usize) -> bool {
    match d {
        0 => r == 0 || h[r - 1][c],
        1 => c == N - 1 || v[r][c],
        2 => r == N - 1 || h[r][c],
        3 => c == 0 || v[r][c - 1],
        _ => unreachable!(),
    }
}

fn simulate(
    v: &[Vec<bool>],
    h: &[Vec<bool>],
    automaton: &[(u8, usize, u8, usize)],
    start_r: usize,
    start_c: usize,
    start_d: usize,
) -> usize {
    let m = automaton.len();
    let state_size = N * N * 4 * m;
    let encode =
        |r: usize, c: usize, d: usize, s: usize| -> usize { ((r * N + c) * 4 + d) * m + s };

    let mut visited = vec![0u32; state_size];
    let mut trajectory: Vec<(usize, usize)> = Vec::new();

    let mut r = start_r;
    let mut c = start_c;
    let mut d = start_d;
    let mut s = 0usize;
    let mut step = 1u32;

    loop {
        let sid = encode(r, c, d, s);
        if visited[sid] != 0 {
            let cycle_start = (visited[sid] - 1) as usize;
            let mut covered = vec![vec![false; N]; N];
            for &(cr, cc) in &trajectory[cycle_start..] {
                covered[cr][cc] = true;
            }
            return covered
                .iter()
                .map(|row| row.iter().filter(|&&x| x).count())
                .sum();
        }
        visited[sid] = step;
        trajectory.push((r, c));

        let wall = has_wall_ahead(v, h, r, c, d);
        let (action, next_s) = if wall {
            (automaton[s].2, automaton[s].3)
        } else {
            (automaton[s].0, automaton[s].1)
        };

        match action {
            0 => {
                r = (r as i32 + DI[d]) as usize;
                c = (c as i32 + DJ[d]) as usize;
            }
            1 => d = turn_right(d),
            2 => d = turn_left(d),
            _ => unreachable!(),
        }
        s = next_s;
        step += 1;

        if step as usize > state_size + 1 {
            break;
        }
    }
    0
}

/// Generate all m-state automata.
/// Each state has: (action_no_wall: 0/1/2, next_no_wall: 0..m-1, action_wall: 1/2, next_wall: 0..m-1)
fn enumerate(m: usize) -> Vec<Vec<(u8, usize, u8, usize)>> {
    // Each state has (3 * m) * (2 * m) = 6m^2 possibilities
    // Total: (6m^2)^m
    let per_state = 6 * m * m;
    let total: usize = per_state.pow(m as u32);

    let mut result = Vec::new();
    for idx in 0..total {
        let mut automaton = Vec::with_capacity(m);
        let mut rem = idx;
        let mut valid = true;
        for _ in 0..m {
            let state_idx = rem % per_state;
            rem /= per_state;

            // Decode: state_idx = (a0 * m + b0) * (2 * m) + (a1_idx * m + b1)
            let wall_part = state_idx % (2 * m);
            let no_wall_part = state_idx / (2 * m);

            let a0 = (no_wall_part / m) as u8; // 0, 1, 2
            let b0 = no_wall_part % m;
            let a1_raw = wall_part / m; // 0 or 1
            let b1 = wall_part % m;
            let a1 = (a1_raw + 1) as u8; // 1=R or 2=L (no F when wall)

            if a0 > 2 || a1 > 2 {
                valid = false;
                break;
            }
            automaton.push((a0, b0, a1, b1));
        }
        if valid {
            result.push(automaton);
        }
    }
    result
}

fn action_char(a: u8) -> char {
    match a {
        0 => 'F',
        1 => 'R',
        2 => 'L',
        _ => '?',
    }
}

fn main() {
    // Read test input files from command line args
    let args: Vec<String> = std::env::args().collect();
    let input_files: Vec<String> = if args.len() > 1 {
        args[1..].to_vec()
    } else {
        // Default: read from stdin, single test case
        vec!["-".to_string()]
    };

    // Read all test grids
    let mut grids: Vec<(Vec<Vec<bool>>, Vec<Vec<bool>>)> = Vec::new();
    for path in &input_files {
        let (v, h) = if path == "-" {
            read_grid_stdin()
        } else {
            read_grid_file(path)
        };
        grids.push((v, h));
    }

    let num_grids = grids.len();
    eprintln!("Loaded {} grids", num_grids);

    // Sampling positions: try a representative subset
    let sample_positions: Vec<(usize, usize, usize)> = {
        let mut pos = Vec::new();
        // For m=2: all positions (fast enough); for m=3: sparse sampling
        for r in (0..N).step_by(2) {
            for c in (0..N).step_by(2) {
                for d in 0..4 {
                    pos.push((r, c, d));
                }
            }
        }
        pos
    };

    // Full positions for refinement of top candidates
    let all_positions: Vec<(usize, usize, usize)> = {
        let mut pos = Vec::new();
        for r in 0..N {
            for c in 0..N {
                for d in 0..4 {
                    pos.push((r, c, d));
                }
            }
        }
        pos
    };

    for m in 2..=3 {
        eprintln!("=== Enumerating {}-state automata ===", m);
        let automata = enumerate(m);
        eprintln!("Total automata: {}", automata.len());

        // Use all positions for m=2 (fast), sparse for m=3
        let positions = if m <= 2 {
            &all_positions
        } else {
            &sample_positions
        };

        // For each automaton, compute best coverage across all grids
        // Store: (avg_best_coverage, automaton_index)
        let mut results: Vec<(f64, usize)> = Vec::new();

        for (ai, automaton) in automata.iter().enumerate() {
            if ai % 10000 == 0 && ai > 0 {
                eprintln!("  Progress: {}/{}", ai, automata.len());
            }

            let mut total_best = 0usize;

            for (v, h) in &grids {
                let mut best_cov = 0usize;
                for &(r, c, d) in positions {
                    let cov = simulate(v, h, automaton, r, c, d);
                    if cov > best_cov {
                        best_cov = cov;
                    }
                }
                total_best += best_cov;
            }

            let avg = total_best as f64 / num_grids as f64;
            let efficiency = avg / m as f64;

            // Only keep if efficiency is interesting (at least 3 cells per state)
            if efficiency >= 3.0 {
                results.push((avg, ai));
            }
        }

        // Sort by average best coverage (descending)
        results.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());

        // For m=3, refine top 100 candidates with all positions
        if m >= 3 {
            let refine_count = results.len().min(100);
            eprintln!(
                "Refining top {} candidates with all positions...",
                refine_count
            );
            let mut refined: Vec<(f64, usize)> = Vec::new();
            for i in 0..refine_count {
                let (_, ai) = results[i];
                let automaton = &automata[ai];
                let mut total_best = 0usize;
                for (v, h) in &grids {
                    let mut best_cov = 0usize;
                    for &(r, c, d) in &all_positions {
                        let cov = simulate(v, h, automaton, r, c, d);
                        if cov > best_cov {
                            best_cov = cov;
                        }
                    }
                    total_best += best_cov;
                }
                let avg = total_best as f64 / num_grids as f64;
                refined.push((avg, ai));
            }
            refined.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
            results = refined;
        }

        // Deduplicate: if two automata have coverage within 0.5 of each other,
        // they might be rotational variants. Keep top unique ones.
        eprintln!("\nTop {}-state automata (coverage / {} states):", m, m);
        let show = results.len().min(30);
        for i in 0..show {
            let (avg, ai) = results[i];
            let automaton = &automata[ai];
            eprint!(
                "  #{:2} avg_cov={:6.1} eff={:5.2} | ",
                i + 1,
                avg,
                avg / m as f64
            );
            for (si, &(a0, b0, a1, b1)) in automaton.iter().enumerate() {
                eprint!(
                    "s{}:[{} {}|{} {}] ",
                    si,
                    action_char(a0),
                    b0,
                    action_char(a1),
                    b1
                );
            }
            eprintln!();
        }

        // Print top 10 as Rust const definitions
        eprintln!("\n// Top {}-state automata as Rust consts:", m);
        let top_n = results.len().min(10);
        for i in 0..top_n {
            let (avg, ai) = results[i];
            let automaton = &automata[ai];
            eprint!(
                "// avg_cov={:.1} | const AUTO_{}_S{}: [(u8, usize, u8, usize); {}] = [",
                avg, i, m, m
            );
            for (si, &(a0, b0, a1, b1)) in automaton.iter().enumerate() {
                if si > 0 {
                    eprint!(", ");
                }
                eprint!("({}, {}, {}, {})", a0, b0, a1, b1);
            }
            eprintln!("];");
        }
        eprintln!();
    }
}

fn read_grid_stdin() -> (Vec<Vec<bool>>, Vec<Vec<bool>>) {
    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();

    let first = lines.next().unwrap().unwrap();
    let _parts: Vec<&str> = first.split_whitespace().collect();
    // N A_K A_M A_W - ignore

    let mut v = Vec::new();
    for _ in 0..N {
        let line = lines.next().unwrap().unwrap();
        v.push(line.trim().chars().map(|c| c == '1').collect());
    }
    let mut h = Vec::new();
    for _ in 0..N - 1 {
        let line = lines.next().unwrap().unwrap();
        h.push(line.trim().chars().map(|c| c == '1').collect());
    }
    (v, h)
}

fn read_grid_file(path: &str) -> (Vec<Vec<bool>>, Vec<Vec<bool>>) {
    let content = std::fs::read_to_string(path).expect(&format!("Cannot read {}", path));
    let mut lines = content.lines();

    let _first = lines.next().unwrap();
    // N A_K A_M A_W - ignore

    let mut v = Vec::new();
    for _ in 0..N {
        let line = lines.next().unwrap();
        v.push(line.trim().chars().map(|c| c == '1').collect());
    }
    let mut h = Vec::new();
    for _ in 0..N - 1 {
        let line = lines.next().unwrap();
        h.push(line.trim().chars().map(|c| c == '1').collect());
    }
    (v, h)
}
