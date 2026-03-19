# Phase 6d: sm.rs ガマットバウンダリ記述

**Status**: IMPLEMENTED
**C版ファイル**: `cmssm.c`（737行）
**Rust見積**: ~200行（impl）+ ~100行（tests）
**ブランチ**: `feat/phase6d-sm`

## Context

Phase 6c で `cmsvirt.c` の移植を完了。`cmssm.c` はガマットバウンダリ記述（GBD）を
実装する自己完結的なモジュール。Jan Morovic の Segment Maxima 法に基づき、
Lab 色空間のサンプル点から 16×16 球面グリッドでガマット境界をモデル化する。

## 変更対象ファイル

| ファイル               | 操作     |
| ---------------------- | -------- |
| `src/transform/sm.rs`  | 新規作成 |
| `src/transform/mod.rs` | mod追加  |

## 実装する関数

| 関数                       | C版                 | 内容                           |
| -------------------------- | ------------------- | ------------------------------ |
| `GamutBoundary::new()`     | `cmsGBDAlloc`       | GBD 割り当て（16×16 グリッド） |
| `GamutBoundary::add_point` | `cmsGDBAddPoint`    | Lab サンプル点をグリッドに追加 |
| `GamutBoundary::compute`   | `cmsGDBCompute`     | 欠損セクタの補間               |
| `GamutBoundary::check`     | `cmsGDBCheckPoint`  | Lab 点がガマット内か判定       |
| `to_spherical`             | `ToSpherical`       | Lab → 球面座標変換             |
| `to_cartesian`             | `ToCartesian`       | 球面座標 → Lab 変換            |
| `closest_line_to_line`     | `ClosestLineToLine` | 2直線の最近接点（3D幾何）      |

## アルゴリズム

```text
1. GBD中心 = Lab(50, 0, 0)（ニュートラルグレー）
2. サンプル点 → 球面座標 (r, alpha, theta) に変換
3. 16×16 グリッド: alpha=色相(360°/16), theta=明度(180°/16)
4. 各セクタは最大半径の点のみ保持（Segment Maxima）
5. Compute: 欠損セクタを隣接セクタから補間
   - 螺旋探索で非空の隣接セクタを発見
   - 中心→セクタ方向の光線と隣接間エッジの交差点を求める
   - 最大半径の交差点を境界値とする
6. Check: 点の半径 ≤ セクタ境界半径 → ガマット内
```

## 既存モジュール依存

| 依存先         | 利用する関数               |
| -------------- | -------------------------- |
| `math/mtrx.rs` | `Vec3::new/dot/length/sub` |
| `types.rs`     | `CieLab`                   |

## コミット構成（TDD）

### Commit 1: 計画書

### Commit 2 (RED): テスト

- `add_point` + `check`: sRGB ガマット内の点 → true
- `add_point` + `check`: ガマット外の点 → false
- 空の GBD → check は false
- `to_spherical` / `to_cartesian` round-trip

### Commit 3 (GREEN): 実装

## 検証方法

```bash
cargo test sm
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt -- --check
```
