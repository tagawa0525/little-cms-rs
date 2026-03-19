# Phase 8: Prelin16 16bit 高速パス

**Status**: PLANNED
**C版ファイル**: `cmsopt.c`（Prelin16Data / PrelinEval16 / OptimizeByResampling 強化: ~250行相当）
**Rust見積**: ~200行（impl）+ ~50行（tests）
**ブランチ**: `feat/phase8-prelin16`

## Context

Phase 7 で 8bit 高速パス（MatShaper8Data, Prelin8Data）を実装した。
本フェーズでは 16bit 入力に対応する汎用高速パス（Prelin16Data）を実装し、
`optimize_by_resampling` を C版相当に強化する。

C版の `OptimizeByResampling` は単純な CLUT リサンプリングではなく、
前後リニアライゼーション曲線を保持し Prelin16Data で高速評価する。
現在の Rust版 `optimize_by_resampling` はこの曲線保持・高速評価が欠落している。

## スコープ

### 実装する機能

1. **Prelin16Data** — 汎用 Curves→CLUT→Curves 16bit 高速評価構造体
2. **PrelinEval16** — 入力曲線→CLUT補間→出力曲線の16bit評価関数
3. **optimize_by_resampling 強化** — 前後曲線保持＋Prelin16Data高速パス＋白色点修正

### Deferred

- FLAGS_CLUT_PRE_LINEARIZATION / FLAGS_CLUT_POST_LINEARIZATION の完全対応
- Lab16入力のCLUT最適化回避（centering issue）
- Float形式入出力の拒否チェック（現状は16bit最適化にフォーマットチェック追加のみ）

## アーキテクチャ設計

### Prelin16Data

```rust
pub(crate) struct Prelin16Data {
    n_inputs: u32,
    n_outputs: u32,
    // Input curves: 1D interp params + table per channel (None = identity)
    curves_in: Vec<Option<(InterpParams, Vec<u16>)>>,
    // CLUT: interp params + table (shared with pipeline stage via pre-fix)
    clut_params: InterpParams,
    clut_table: Vec<u16>,
    // Output curves: 1D interp params + table per channel (None = identity)
    curves_out: Vec<Option<(InterpParams, Vec<u16>)>>,
}
```

### 評価フロー

```text
Input[nInputs]
  → curves_in[i]: 1D lerp16 per channel (or identity)
  → CLUT: nD tetrahedral/trilinear interp
  → curves_out[i]: 1D lerp16 per channel (or identity)
→ Output[nOutputs]
```

### FastEval16 拡張

```rust
pub(crate) enum FastEval16 {
    MatShaper(Box<MatShaper8Data>),
    Prelin8(Box<Prelin8Data>),
    Prelin16(Box<Prelin16Data>),   // NEW
}
```

## 変更対象ファイル

| ファイル               | 操作 |
| ---------------------- | ---- |
| `src/pipeline/lut.rs`  | 修正 |
| `src/transform/opt.rs` | 修正 |

## 実装する関数

### Prelin16Data

- `Prelin16Data` — 構造体定義
- `prelin_eval16()` — 16bit評価（C版: `PrelinEval16`）
- `prelin16_alloc()` — 構築ヘルパー（C版: `PrelinOpt16alloc`）

### optimize_by_resampling 強化

- 前後曲線の検出・保持（FLAGS_CLUT_PRE/POST_LINEARIZATION）
- CLUT サンプリング（曲線除去後のパイプラインで評価）
- Prelin16Data 構築・FastEval16 設定
- FixWhiteMisalignment 呼び出し
- format パラメータ追加

## コミット構成（TDD）

### Commit 1: 計画書

### Commit 2 (RED): Prelin16Data テスト

- Curves→CLUT→Curves パイプラインが Prelin16 高速パスに最適化されることを検証
- 最適化前後の出力一致を検証

### Commit 3 (GREEN): Prelin16Data + optimize_by_resampling 強化

## 検証方法

```bash
cargo test opt
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt -- --check
```
