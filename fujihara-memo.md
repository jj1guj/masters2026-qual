指定できるパラメータが三種類
- ロボットの台数
- 各ロボットの内部状態
- 各ロボットの初期配置

問題の特徴
A: ロボット無料
B: 壁が安い
C: 頑張る

# A

ロボットを増やすと状態が増えるのでコスト的には高くつく。
あるロボットの状態を増やして、結果的に台数が減るならコストは減らせる。

いい感じにうねうねするオートマトンでざっくりカバーし、漏らしたエリアにロボットを置く。→417253948

2,3状態のオートマトンを全列挙してシミュレート

## 現状の実装（515M）

### 全体構成
1. **オートマトン候補の事前計算**（ビットマスク化）→ 貪欲選択（exact VC）→ SA（fast VC estimate）→ 残りを頂点被覆で処理

### オートマトン候補（8種類）
| 名前 | 状態数 | 平均カバー | 効率(cov/states) | 説明 |
|------|--------|-----------|------------------|------|
| AUTO_2S_A | 2 | 215 | 107.6 | L→1,R→1 / F→0,L→1 |
| AUTO_2S_B | 2 | 215 | 107.6 | AUTO_2S_Aの鏡像 |
| AUTO_2S_C | 2 | 209 | 104.5 | R→1,L→1 / F→0,R→1 |
| AUTO_2S_D | 2 | 209 | 104.5 | AUTO_2S_Cの鏡像 |
| AUTO_3S_A | 3 | 286 | 95.3 | 3状態列挙で発見 |
| AUTO_3S_B | 3 | 286 | 95.3 | AUTO_3S_Aの鏡像 |
| SNAKE | 6 | ~300 | ~50 | 蛇行パターン(チームメイト考案) |
| REV_SNAKE | 6 | ~300 | ~50 | SNAKEの鏡像 |

### 全候補の事前計算
- 8種類 × 20×20位置 × 4方向 = 12,800候補
- 各候補について simulate_automaton で周期的カバーセルを計算
- カバレッジを `[u32; 20]` ビットマスクで保持（高速OR演算用）
- 状態空間: (r, c, d, s) → 訪問済み管理で周期検出

### 貪欲選択（Greedy multi-snake, exact VC）
- 初期: pure VCのコストを計算
- ループ: 全候補から「snake_states + auto_states + VC残りコスト」が最小になるものを選択
  - 枝刈り1: ビットマスクでnew_cells < 3 の候補はスキップ
  - 枝刈り2: fast_vc_estimate（近似）で best_total を超える候補をスキップ
  - 通過した候補のみ vertex_cover_for_uncovered（exact max-flow）を呼ぶ
- 改善がなくなるまで繰り返し

### 焼きなまし法（SA, fast VC estimate）
- 初期解: greedy の結果
- 近傍操作: add / remove / replace（ランダムに1つ）
- 評価関数: snake_states + fast_vc_estimate（O(N²)の近似VC）
  - exact VCを毎回呼ぶと~80K iter/sだが、近似なら~1.8M iter/s
- 温度: t_start=5.0, t_end=0.1, 指数冷却
- 時間制限: 800ms（greedy部分で0.2〜0.5秒使うため）
- 最良解を記録し、最終出力時にexact VCで正確なコストを計算
- rand 0.9.2（AtCoder judge互換: random_range, random）

### 頂点被覆（Vertex Cover for uncovered cells）
- snakeでカバーされなかったセルを対象
- **2モード比較**: 両方試して安い方を返す
  - "Broken" segments: 壁 AND カバー済みセルで分断（細かいセグメント）
  - "Mega" segments: 壁のみで分断（カバー済みセルを含むが、連続セグメントのコスト削減）
- 二部グラフの最小重み頂点被覆（max-flow / min-cut で解く）
  - segment長1 → 1状態spinner、segment長2以上 → 2状態U-turnロボット
  - ac-library-rs の MfGraph を使用
- König の定理: 二部グラフの最小頂点被覆 = 最大マッチング

### fast_vc_estimate（SA内で使用する近似VC）
- max-flowなしでO(N²)で計算
- 行セグメント総コストと列セグメント総コストのmin（VCの上界）
- broken/megaの両モードを試してmin
- 正確なVCより若干大きい値だが、相対順序はほぼ正しい

### 壁
- 壁の追加コスト A_W=1000 なので壁は一切追加しない（output_no_walls）

### スコア推移
- 全セルにspinner配置: ~150M（ベースライン）
- row segment往復: 240M
- row/col選択: 261M
- 最小重み頂点被覆: 282M
- 6状態snake + VC: 417M
- マルチsnake(正逆): 437M
- 2状態/3状態オートマトン追加: 510M
- swap最適化追加: 512M
- SA（exact VC）: 513M
- メガセグメントVC追加: 515M
- SA高速化（fast VC estimate + ビットマスク）: **515M**

### 改善候補
- 4状態オートマトン探索（84M通りだが事前フィルタで削減可能）
- SA温度パラメータチューニング（deltaの統計取って調整）
- fast_vc_estimateの精度改善（行列交差部分の考慮）
- greedy初期解のランダム化（複数回やってSA開始点を変える）

# C


