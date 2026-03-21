#!/usr/bin/env bash
set -euo pipefail

# Usage:
#   ./scripts/annealer_multi.sh [input_dir] [seconds_per_run] [num_runs] [parallel_jobs]
# Example:
#   ./scripts/annealer_multi.sh ../tools/inB 120 12 4

INPUT_DIR="${1:-../tools/inB}"
SECS="${2:-120}"
RUNS="${3:-8}"
JOBS="${4:-}"

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
OUT_DIR="$ROOT_DIR/../results/annealer_multi"
mkdir -p "$OUT_DIR"

if [[ -z "$JOBS" ]]; then
  if command -v sysctl >/dev/null 2>&1; then
    JOBS="$(sysctl -n hw.logicalcpu 2>/dev/null || echo 1)"
  else
    JOBS=1
  fi
fi

if ! [[ "$JOBS" =~ ^[0-9]+$ ]] || (( JOBS <= 0 )); then
  echo "[warn] invalid parallel_jobs=$JOBS; fallback to 1"
  JOBS=1
fi
if (( JOBS > RUNS )); then
  JOBS=$RUNS
fi

if ! [[ "$RUNS" =~ ^[0-9]+$ ]] || (( RUNS <= 0 )); then
  echo "[error] num_runs must be positive integer"
  exit 1
fi

if ! [[ "$SECS" =~ ^[0-9]+$ ]] || (( SECS <= 0 )); then
  echo "[error] seconds_per_run must be positive integer"
  exit 1
fi

echo "[info] building annealer binary..."
(
  cd "$ROOT_DIR"
  cargo build --release --bin annealer >/dev/null
)
BIN="$ROOT_DIR/../target/release/annealer"

if [[ ! -x "$BIN" ]]; then
  echo "[error] annealer binary not found: $BIN"
  exit 1
fi

best_full=-1
best_score=-1
best_seed=""
best_ans=""
best_log=""

echo "[info] input_dir=$INPUT_DIR secs=$SECS runs=$RUNS jobs=$JOBS"

declare -a pids
declare -a seeds
declare -a ans_files
declare -a log_files

for ((i=1; i<=RUNS; i++)); do
  seed=$(( $(date +%s%N) ^ (i * 1000003) ))
  ans_file="$OUT_DIR/answer_${i}.txt"
  log_file="$OUT_DIR/log_${i}.txt"
  seeds[$i]="$seed"
  ans_files[$i]="$ans_file"
  log_files[$i]="$log_file"

  echo "[run $i/$RUNS] seed=$seed (start)"
  (
    cd "$ROOT_DIR"
    "$BIN" "$INPUT_DIR" "$SECS" "$seed" \
      >"$ans_file" 2>"$log_file"
  ) &
  pids[$i]=$!

  if (( i % JOBS == 0 )) || (( i == RUNS )); then
    echo "[info] waiting for batch to finish..."
    batch_start=$(( i - JOBS + 1 ))
    if (( batch_start < 1 )); then
      batch_start=1
    fi
    for ((j=batch_start; j<=i; j++)); do
      pid="${pids[$j]:-}"
      if [[ -z "$pid" ]]; then
        continue
      fi
      if wait "$pid"; then
        echo "[run $j/$RUNS] seed=${seeds[$j]} (done)"
      else
        echo "[run $j/$RUNS] seed=${seeds[$j]} (failed)"
      fi
    done
  fi
done

for ((i=1; i<=RUNS; i++)); do
  seed="${seeds[$i]}"
  ans_file="${ans_files[$i]}"
  log_file="${log_files[$i]}"

  summary_line="$(grep '完了:' "$log_file" | tail -1 || true)"
  full="$(echo "$summary_line" | sed -E -n 's/.*全カバー: ([0-9]+)\/[0-9]+.*/\1/p')"
  score="$(echo "$summary_line" | sed -E -n 's/.*ベストスコア=([0-9]+) \/ [0-9]+.*/\1/p')"

  if [[ -z "$full" ]]; then full=0; fi
  if [[ -z "$score" ]]; then score=0; fi

  echo "[run $i/$RUNS] seed=$seed -> full=${full}/100 score=${score}"

  if (( full > best_full )) || { (( full == best_full )) && (( score > best_score )); }; then
    best_full=$full
    best_score=$score
    best_seed=$seed
    best_ans="$ans_file"
    best_log="$log_file"
  fi
done

echo ""
echo "[best] seed=$best_seed full=${best_full}/100 score=$best_score"
echo "[best] answer=$best_ans"
echo "[best] log=$best_log"

cp "$best_ans" "$OUT_DIR/best_answer.txt"
cp "$best_log" "$OUT_DIR/best_log.txt"
echo "[saved] $OUT_DIR/best_answer.txt"
echo "[saved] $OUT_DIR/best_log.txt"
