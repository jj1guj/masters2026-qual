use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use std::process::Command;

const INDEX_HTML: &str = include_str!("../static/index.html");

async fn index() -> impl Responder {
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(INDEX_HTML)
}

#[derive(Deserialize)]
struct RunSolverRequest {
    input_data: String,
}

#[derive(Serialize)]
struct RunSolverResponse {
    success: bool,
    output: String,
    stderr: String,
}

async fn run_solver(req: web::Json<RunSolverRequest>) -> impl Responder {
    // Find workspace root (parent of visualizer/)
    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();

    let solver_manifest = workspace_root.join("solver/Cargo.toml");

    // Build the solver
    let build_result = Command::new("cargo")
        .args(["build", "--release", "--manifest-path"])
        .arg(&solver_manifest)
        .output();

    match build_result {
        Ok(output) if !output.status.success() => {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            return HttpResponse::Ok().json(RunSolverResponse {
                success: false,
                output: String::new(),
                stderr: format!("Build failed:\n{}", stderr),
            });
        }
        Err(e) => {
            return HttpResponse::Ok().json(RunSolverResponse {
                success: false,
                output: String::new(),
                stderr: format!("Failed to run cargo: {}", e),
            });
        }
        _ => {}
    }

    // Run the solver
    let solver_binary = workspace_root.join("target/release/solver");
    let run_result = Command::new(&solver_binary)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn();

    match run_result {
        Ok(mut child) => {
            use std::io::Write;
            if let Some(ref mut stdin) = child.stdin {
                let _ = stdin.write_all(req.input_data.as_bytes());
            }
            match child.wait_with_output() {
                Ok(output) => {
                    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                    HttpResponse::Ok().json(RunSolverResponse {
                        success: output.status.success(),
                        output: stdout,
                        stderr,
                    })
                }
                Err(e) => HttpResponse::Ok().json(RunSolverResponse {
                    success: false,
                    output: String::new(),
                    stderr: format!("Failed to wait for solver: {}", e),
                }),
            }
        }
        Err(e) => HttpResponse::Ok().json(RunSolverResponse {
            success: false,
            output: String::new(),
            stderr: format!("Failed to run solver: {}", e),
        }),
    }
}

#[derive(Deserialize)]
struct ScoreRequest {
    input_data: String,
    output_data: String,
}

#[derive(Serialize)]
struct ScoreResponse {
    success: bool,
    score: i64,
    error: String,
    checked: Vec<Vec<bool>>,
    routes: Vec<RouteInfo>,
    wall_v_combined: Vec<Vec<u8>>,
    wall_h_combined: Vec<Vec<u8>>,
    robots: Vec<RobotInfo>,
    n: usize,
    ak: i64,
    am: i64,
    aw: i64,
    k: usize,
    m_total: usize,
    w: usize,
    v_cost: i64,
}

#[derive(Serialize)]
struct RouteInfo {
    head: Vec<(usize, usize)>,
    tail: Vec<(usize, usize)>,
}

#[derive(Serialize)]
struct RobotInfo {
    i: usize,
    j: usize,
    d: usize,
    m: usize,
}

async fn calculate_score(req: web::Json<ScoreRequest>) -> impl Responder {
    let input = parse_input_data(&req.input_data);
    let input = match input {
        Ok(v) => v,
        Err(e) => {
            return HttpResponse::Ok().json(ScoreResponse {
                success: false,
                score: 0,
                error: e,
                checked: vec![],
                routes: vec![],
                wall_v_combined: vec![],
                wall_h_combined: vec![],
                robots: vec![],
                n: 0,
                ak: 0,
                am: 0,
                aw: 0,
                k: 0,
                m_total: 0,
                w: 0,
                v_cost: 0,
            });
        }
    };

    let output = parse_output_data(&input, &req.output_data);
    let output = match output {
        Ok(v) => v,
        Err(e) => {
            return HttpResponse::Ok().json(ScoreResponse {
                success: false,
                score: 0,
                error: e,
                checked: vec![],
                routes: vec![],
                wall_v_combined: vec![],
                wall_h_combined: vec![],
                robots: vec![],
                n: input.n,
                ak: input.ak,
                am: input.am,
                aw: input.aw,
                k: 0,
                m_total: 0,
                w: 0,
                v_cost: 0,
            });
        }
    };

    // Compute combined walls
    let mut wall_v_combined = vec![vec![0u8; input.n - 1]; input.n];
    let mut wall_h_combined = vec![vec![0u8; input.n]; input.n - 1];
    for i in 0..input.n {
        for j in 0..input.n - 1 {
            if input.wall_v[i][j] == '1' || output.wall_v[i][j] == '1' {
                wall_v_combined[i][j] = 1;
            }
        }
    }
    for i in 0..input.n - 1 {
        for j in 0..input.n {
            if input.wall_h[i][j] == '1' || output.wall_h[i][j] == '1' {
                wall_h_combined[i][j] = 1;
            }
        }
    }

    // Simulate robots
    let (score, error, checked, routes) =
        compute_score_details(&input, &output, &wall_v_combined, &wall_h_combined);

    let route_infos: Vec<RouteInfo> = routes
        .iter()
        .map(|r| RouteInfo {
            head: r.head.clone(),
            tail: r.tail.clone(),
        })
        .collect();

    let robot_infos: Vec<RobotInfo> = output
        .robots
        .iter()
        .map(|r| RobotInfo {
            i: r.i,
            j: r.j,
            d: r.d,
            m: r.m,
        })
        .collect();

    let k = output.robots.len();
    let m_total: usize = output.robots.iter().map(|r| r.m).sum();
    let w: usize = output
        .wall_v
        .iter()
        .map(|r| r.iter().filter(|&&c| c == '1').count())
        .sum::<usize>()
        + output
            .wall_h
            .iter()
            .map(|r| r.iter().filter(|&&c| c == '1').count())
            .sum::<usize>();
    let v_cost = input.ak * (k as i64 - 1) + input.am * m_total as i64 + input.aw * w as i64;

    HttpResponse::Ok().json(ScoreResponse {
        success: error.is_empty(),
        score,
        error,
        checked,
        routes: route_infos,
        wall_v_combined,
        wall_h_combined,
        robots: robot_infos,
        n: input.n,
        ak: input.ak,
        am: input.am,
        aw: input.aw,
        k,
        m_total,
        w,
        v_cost,
    })
}

// ---- Problem-specific parsing and scoring logic ----

struct InputData {
    n: usize,
    ak: i64,
    am: i64,
    aw: i64,
    wall_v: Vec<Vec<char>>,
    wall_h: Vec<Vec<char>>,
}

struct Robot {
    m: usize,
    i: usize,
    j: usize,
    d: usize,
    a0: Vec<char>,
    b0: Vec<usize>,
    a1: Vec<char>,
    b1: Vec<usize>,
}

struct OutputData {
    robots: Vec<Robot>,
    wall_v: Vec<Vec<char>>,
    wall_h: Vec<Vec<char>>,
}

#[derive(Clone)]
struct Route {
    head: Vec<(usize, usize)>,
    tail: Vec<(usize, usize)>,
}

const DIR: [char; 4] = ['U', 'R', 'D', 'L'];
const DIJ: [(usize, usize); 4] = [(!0, 0), (0, 1), (1, 0), (0, !0)];

fn parse_input_data(f: &str) -> Result<InputData, String> {
    let mut tokens = f.split_whitespace();
    let n: usize = tokens
        .next()
        .ok_or("Missing N")?
        .parse()
        .map_err(|_| "Parse error N")?;
    let ak: i64 = tokens
        .next()
        .ok_or("Missing AK")?
        .parse()
        .map_err(|_| "Parse error AK")?;
    let am: i64 = tokens
        .next()
        .ok_or("Missing AM")?
        .parse()
        .map_err(|_| "Parse error AM")?;
    let aw: i64 = tokens
        .next()
        .ok_or("Missing AW")?
        .parse()
        .map_err(|_| "Parse error AW")?;

    let mut wall_v = vec![];
    for _ in 0..n {
        let line = tokens.next().ok_or("Missing wall_v line")?;
        if line.len() != n - 1 {
            return Err(format!("Invalid wall_v length: {} (expected {})", line.len(), n - 1));
        }
        wall_v.push(line.chars().collect());
    }
    let mut wall_h = vec![];
    for _ in 0..n - 1 {
        let line = tokens.next().ok_or("Missing wall_h line")?;
        if line.len() != n {
            return Err(format!("Invalid wall_h length: {} (expected {})", line.len(), n));
        }
        wall_h.push(line.chars().collect());
    }
    Ok(InputData {
        n,
        ak,
        am,
        aw,
        wall_v,
        wall_h,
    })
}

fn parse_output_data(input: &InputData, f: &str) -> Result<OutputData, String> {
    let mut tokens = f.split_whitespace();
    let k: usize = tokens
        .next()
        .ok_or("Missing K")?
        .parse()
        .map_err(|_| "Parse error K")?;
    if k < 1 || k > input.n * input.n {
        return Err(format!("K out of range: {}", k));
    }

    let mut robots = vec![];
    for _ in 0..k {
        let m: usize = tokens
            .next()
            .ok_or("Missing m")?
            .parse()
            .map_err(|_| "Parse error m")?;
        if m < 1 || m > 4 * input.n * input.n {
            return Err(format!("m out of range: {}", m));
        }
        let i: usize = tokens
            .next()
            .ok_or("Missing i")?
            .parse()
            .map_err(|_| "Parse error i")?;
        let j: usize = tokens
            .next()
            .ok_or("Missing j")?
            .parse()
            .map_err(|_| "Parse error j")?;
        let d_char: char = tokens
            .next()
            .ok_or("Missing d")?
            .chars()
            .next()
            .ok_or("Empty d")?;
        let d = DIR
            .iter()
            .position(|&c| c == d_char)
            .ok_or(format!("Invalid direction: {}", d_char))?;

        let mut a0 = vec![];
        let mut b0 = vec![];
        let mut a1 = vec![];
        let mut b1 = vec![];
        for _ in 0..m {
            let a: char = tokens
                .next()
                .ok_or("Missing a0")?
                .chars()
                .next()
                .ok_or("Empty a0")?;
            if !['R', 'L', 'F'].contains(&a) {
                return Err(format!("Invalid action a0: {}", a));
            }
            a0.push(a);
            let b: usize = tokens
                .next()
                .ok_or("Missing b0")?
                .parse()
                .map_err(|_| "Parse error b0")?;
            if b >= m {
                return Err(format!("b0 out of range: {}", b));
            }
            b0.push(b);
            let a: char = tokens
                .next()
                .ok_or("Missing a1")?
                .chars()
                .next()
                .ok_or("Empty a1")?;
            if !['R', 'L'].contains(&a) {
                return Err(format!("Invalid action a1: {}", a));
            }
            a1.push(a);
            let b: usize = tokens
                .next()
                .ok_or("Missing b1")?
                .parse()
                .map_err(|_| "Parse error b1")?;
            if b >= m {
                return Err(format!("b1 out of range: {}", b));
            }
            b1.push(b);
        }
        robots.push(Robot {
            m,
            i,
            j,
            d,
            a0,
            b0,
            a1,
            b1,
        });
    }

    let mut wall_v = vec![];
    for _ in 0..input.n {
        let line = tokens.next().ok_or("Missing output wall_v line")?;
        if line.len() != input.n - 1 {
            return Err(format!("Invalid output wall_v length: {}", line.len()));
        }
        wall_v.push(line.chars().collect());
    }
    let mut wall_h = vec![];
    for _ in 0..input.n - 1 {
        let line = tokens.next().ok_or("Missing output wall_h line")?;
        if line.len() != input.n {
            return Err(format!("Invalid output wall_h length: {}", line.len()));
        }
        wall_h.push(line.chars().collect());
    }

    Ok(OutputData {
        robots,
        wall_v,
        wall_h,
    })
}

fn has_wall(wall_v: &[Vec<u8>], wall_h: &[Vec<u8>], i: usize, j: usize, d: usize) -> bool {
    let n = wall_v.len();
    let i2 = i.wrapping_add(DIJ[d].0);
    let j2 = j.wrapping_add(DIJ[d].1);
    if i2 >= n || j2 >= n {
        return true;
    }
    if i == i2 {
        wall_v[i][j.min(j2)] == 1
    } else {
        wall_h[i.min(i2)][j] == 1
    }
}

fn compute_score_details(
    input: &InputData,
    out: &OutputData,
    wall_v_combined: &[Vec<u8>],
    wall_h_combined: &[Vec<u8>],
) -> (i64, String, Vec<Vec<bool>>, Vec<Route>) {
    let n = input.n;
    let mut routes = vec![];

    for robot in &out.robots {
        // visited[i][j][d][s] = first time visited in this config
        let mut visited = vec![vec![vec![vec![usize::MAX; robot.m]; 4]; n]; n];
        let mut route = vec![];
        let mut ci = robot.i;
        let mut cj = robot.j;
        let mut cd = robot.d;
        let mut cs = 0usize;
        for t in 0.. {
            route.push((ci, cj));
            if visited[ci][cj][cd][cs] != usize::MAX {
                let loop_start = visited[ci][cj][cd][cs];
                let head = route[..=loop_start].to_vec();
                let tail = route[loop_start..].to_vec();
                routes.push(Route { head, tail });
                break;
            }
            visited[ci][cj][cd][cs] = t;

            let wall_front = has_wall(wall_v_combined, wall_h_combined, ci, cj, cd);
            let (a, b) = if wall_front {
                (robot.a1[cs], robot.b1[cs])
            } else {
                (robot.a0[cs], robot.b0[cs])
            };
            match a {
                'R' => cd = (cd + 1) % 4,
                'L' => cd = (cd + 3) % 4,
                'F' => {
                    ci = ci.wrapping_add(DIJ[cd].0);
                    cj = cj.wrapping_add(DIJ[cd].1);
                }
                _ => {}
            }
            cs = b;
        }
    }

    let mut checked = vec![vec![false; n]; n];
    let mut num = 0;
    for route in &routes {
        for &(i, j) in &route.tail {
            if !checked[i][j] {
                checked[i][j] = true;
                num += 1;
            }
        }
    }

    if num != n * n {
        return (
            0,
            format!("Not all cells are patrolled: {}/{}", num, n * n),
            checked,
            routes,
        );
    }

    let k = out.robots.len() as i64;
    let m_total: i64 = out.robots.iter().map(|r| r.m as i64).sum();
    let w: i64 = out
        .wall_v
        .iter()
        .map(|r| r.iter().filter(|&&c| c == '1').count() as i64)
        .sum::<i64>()
        + out
            .wall_h
            .iter()
            .map(|r| r.iter().filter(|&&c| c == '1').count() as i64)
            .sum::<i64>();

    let v = input.ak * (k - 1) + input.am * m_total + input.aw * w;
    let base = input.ak * (n * n - 1) as i64 + input.am * (n * n) as i64;
    let mut score = 1i64;
    if v > 0 && base > 0 && v < base {
        score += (1e6 * (base as f64 / v as f64).log2()).round() as i64;
    }

    (score, String::new(), checked, routes)
}

// ---- Input generation (replicates tools/src/lib.rs gen()) ----

#[derive(Deserialize)]
struct GenerateRequest {
    seed: u64,
    problem: String,
}

#[derive(Serialize)]
struct GenerateResponse {
    success: bool,
    input_data: String,
    error: String,
}

fn gen_input(seed: u64, problem: &str) -> Result<String, String> {
    let mut rng = rand_chacha::ChaCha20Rng::seed_from_u64(seed);
    let n = 20usize;
    let (ak, am, aw): (i64, i64, i64) = match problem {
        "A" => (0, 1, 1000),
        "B" => (1000, rng.gen_range(1..=10), rng.gen_range(1..=10)),
        "C" => (1000, 1, 1000),
        _ => return Err(format!("Invalid problem id: {}", problem)),
    };
    let x = rng.gen_range(1..=6i32);
    let y = rng.gen_range(1..=6i32);
    loop {
        let mut wall_v = vec![vec!['0'; n - 1]; n];
        let mut wall_h = vec![vec!['0'; n]; n - 1];
        for _ in 0..x {
            let dir = rng.gen_range(0..2i32);
            let l = rng.gen_range(5..=15i32) as usize;
            let i = rng.gen_range(0..n as i32) as usize;
            let j = rng.gen_range(0..n as i32 - 1) as usize;
            if dir == 0 {
                for i in (i + 1).saturating_sub(l)..=i {
                    wall_v[i][j] = '1';
                }
            } else {
                for i in i..=(i + l - 1).min(n - 1) {
                    wall_v[i][j] = '1';
                }
            }
        }
        for _ in 0..y {
            let dir = rng.gen_range(0..2i32);
            let l = rng.gen_range(5..=15i32) as usize;
            let i = rng.gen_range(0..n as i32 - 1) as usize;
            let j = rng.gen_range(0..n as i32) as usize;
            if dir == 0 {
                for j in (j + 1).saturating_sub(l)..=j {
                    wall_h[i][j] = '1';
                }
            } else {
                for j in j..=(j + l - 1).min(n - 1) {
                    wall_h[i][j] = '1';
                }
            }
        }
        // Connectivity check via DFS
        let mut visited = vec![vec![false; n]; n];
        let mut stack = vec![(0usize, 0usize)];
        visited[0][0] = true;
        let mut num = 0;
        while let Some((i, j)) = stack.pop() {
            num += 1;
            for d in 0..4 {
                if !has_wall_char(&wall_v, &wall_h, i, j, d) {
                    let i2 = i.wrapping_add(DIJ[d].0);
                    let j2 = j.wrapping_add(DIJ[d].1);
                    if i2 < n && j2 < n && !visited[i2][j2] {
                        visited[i2][j2] = true;
                        stack.push((i2, j2));
                    }
                }
            }
        }
        if num == n * n {
            let mut out = String::new();
            out.push_str(&format!("{} {} {} {}\n", n, ak, am, aw));
            for i in 0..n {
                out.push_str(&wall_v[i].iter().collect::<String>());
                out.push('\n');
            }
            for i in 0..n - 1 {
                out.push_str(&wall_h[i].iter().collect::<String>());
                out.push('\n');
            }
            return Ok(out);
        }
    }
}

fn has_wall_char(wall_v: &[Vec<char>], wall_h: &[Vec<char>], i: usize, j: usize, d: usize) -> bool {
    let n = wall_v.len();
    let i2 = i.wrapping_add(DIJ[d].0);
    let j2 = j.wrapping_add(DIJ[d].1);
    if i2 >= n || j2 >= n {
        return true;
    }
    if i == i2 {
        wall_v[i][j.min(j2)] == '1'
    } else {
        wall_h[i.min(i2)][j] == '1'
    }
}

async fn generate_input(req: web::Json<GenerateRequest>) -> impl Responder {
    match gen_input(req.seed, &req.problem) {
        Ok(input_data) => HttpResponse::Ok().json(GenerateResponse {
            success: true,
            input_data,
            error: String::new(),
        }),
        Err(e) => HttpResponse::Ok().json(GenerateResponse {
            success: false,
            input_data: String::new(),
            error: e,
        }),
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let port = 8088;
    println!("Visualizer running at http://localhost:{}", port);

    HttpServer::new(|| {
        App::new()
            .route("/", web::get().to(index))
            .route("/api/run-solver", web::post().to(run_solver))
            .route("/api/score", web::post().to(calculate_score))
            .route("/api/generate", web::post().to(generate_input))
    })
    .bind(("127.0.0.1", port))?
    .run()
    .await
}
