# Phase 7: パイプライン最適化高速パス

**Status**: PLANNED
**C版ファイル**: `cmsopt.c`（追加部分: ~800行相当）
**Rust見積**: ~400行（impl）+ ~150行（tests）
**ブランチ**: `feat/phase7-opt-fast-paths`

## Context

Phase 5d で基本的な最適化（inverse pair 除去、隣接行列結合、カーブ結合、CLUT リサンプリング）を
実装した。本フェーズでは残りの3つの最適化パスを実装し、実用的な変換パフォーマンスを実現する。

## スコープ

### 実装する機能

1. **FixWhiteMisalignment** — CLUT の白色点ノードを補正し、白→白マッピングを保証
2. **OptimizeMatrixShaper** — matrix-shaper パイプラインの 1.14 固定小数点高速パス
3. **OptimizeByComputingLinearization** — CLUT 前後のリニアライゼーション曲線抽出 + 8bit 高速パス

### Deferred

- Prelin16Data（16bit 高速パス）— 8bit パスのみ実装
- プラグインによるカスタム最適化関数登録
- FLAGS_NOWHITEONWHITEFIXUP の完全対応（フラグチェックのみ実装）

## アーキテクチャ設計

### 高速評価パス（FastEval16）

Pipeline に `fast_eval16: Option<FastEval16>` フィールドを追加。
`eval_16()` 呼び出し時、高速パスがあればそれを使用する。

```rust
pub(crate) enum FastEval16 {
    MatShaper(MatShaper8Data),
    Prelin8(Prelin8Data),
}
```

### 1.14 固定小数点

MatShaper で使用。i32 型で表現（C版と同様、16bit 以上の精度が必要）。

- 変換: `DOUBLE_TO_1FIXED14(x) = floor(x * 16384.0 + 0.5) as i32`
- 入力 LUT: 256 エントリ（8bit → 1.14 固定）
- 出力 LUT: 16385 エントリ（1.14 → 16bit）
- 行列乗算: i32 積 + 0x2000（丸め）後、14bit 右シフト

## 変更対象ファイル

| ファイル                | 操作 |
| ----------------------- | ---- |
| `src/transform/opt.rs`  | 修正 |
| `src/pipeline/lut.rs`   | 修正 |
| `src/curves/gamma.rs`   | 修正 |
| `src/math/pcs.rs`       | 参照 |

## 実装する関数

### FixWhiteMisalignment

- `fix_white_misalignment()` — CLUT 白色点ノード補正（C版: `FixWhiteMisalignment`）
- `patch_lut()` — CLUT グリッドノード書き換え（C版: `PatchLUT`）

### OptimizeMatrixShaper

- `optimize_by_matrix_shaper()` — 検出・構築（C版: `OptimizeMatrixShaper`）
- `MatShaper8Data` — 事前計算テーブル構造体
- `MatShaper8Data::eval()` — 1.14 固定小数点評価（C版: `MatShaperEval16`）
- `fill_first_shaper()` — 入力カーブ → 1.14 LUT
- `fill_second_shaper()` — 出力カーブ → 16bit LUT

### OptimizeByComputingLinearization

- `optimize_by_computing_linearization()` — 検出・抽出・構築（C版: `OptimizeByComputingLinearization`）
- `Prelin8Data` — 事前計算テーブル構造体
- `Prelin8Data::eval()` — 8bit tetrahedral 補間（C版: `PrelinEval8`）
- `slope_limiting()` — 曲線端点の傾き制限（C版: `SlopeLimiting`）

### 基盤変更

- `Pipeline::fast_eval16` フィールド追加
- `Pipeline::eval_16()` の高速パス分岐
- `ToneCurve::table16_mut()` — SlopeLimiting 用可変アクセサ

## コミット構成（TDD）

### Commit 1: 計画書

### Commit 2 (RED): FixWhiteMisalignment テスト

- CLUT パイプラインの白色点が補正されることを検証
- 白色点が既に正しい場合は変更なしを検証

### Commit 3 (GREEN): FixWhiteMisalignment 実装

### Commit 4 (RED): OptimizeMatrixShaper テスト

- matrix-shaper パイプラインが MatShaper 高速パスに最適化されることを検証
- 最適化前後の出力一致を検証
- RGB 以外は最適化されないことを検証

### Commit 5 (GREEN): OptimizeMatrixShaper 実装 + FastEval16 基盤

### Commit 6 (RED): OptimizeByComputingLinearization テスト

- CLUT パイプラインからリニアライゼーション抽出を検証
- 最適化前後の出力一致を検証

### Commit 7 (GREEN): OptimizeByComputingLinearization 実装

## 検証方法

```bash
cargo test opt
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt -- --check
```
