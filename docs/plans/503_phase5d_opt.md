# Phase 5d: opt.rs パイプライン最適化

**Status**: PLANNED
**C版ファイル**: `cmsopt.c`（1,992行）
**Rust見積**: ~600行（impl）+ ~200行（tests）
**ブランチ**: `feat/phase5d-opt`

## Context

Phase 5c (samp.rs: 黒点検出 & BPC) 完了。変換パイプラインは動作するが、最適化なし。
`cmsopt.c` はパイプラインの前処理クリーンアップと4種の最適化戦略を実装する。
パイプライン最適化は性能に直結し、冗長なステージ除去は正確性にも影響する。

## 変更対象ファイル

| ファイル | 操作 |
| --- | --- |
| `src/transform/opt.rs` | 新規: パイプライン最適化 |
| `src/transform/mod.rs` | `pub mod opt;` 追加 |
| `src/transform/xform.rs` | `optimize_pipeline()` 呼び出し追加、フラグ定数追加 |
| `src/math/pcs.rs` | `reasonable_gridpoints_by_colorspace()` 追加 |

## 実装する関数

### opt.rs — メインエントリ

| 関数 | C版 | 内容 |
| --- | --- | --- |
| `optimize_pipeline()` | `_cmsOptimizePipeline` | メインエントリ。FLAGS_NOOPTIMIZE チェック → pre_optimize → 各戦略を順次試行 |

### opt.rs — 前処理クリーンアップ

| 関数 | C版 | 内容 |
| --- | --- | --- |
| `pre_optimize()` | `PreOptimize` | 冗長ステージ除去 + 行列結合 |
| `remove_identity_stages()` | (PreOptimize内) | IdentityElem ステージ除去 |
| `remove_inverse_pairs()` | `_Remove2Op` | 隣接逆変換ペア除去 (XYZ↔Lab, LabV2↔V4, FloatPCS) |
| `multiply_adjacent_matrices()` | `_MultiplyMatrix` | 隣接 MatrixElem を行列積で結合 |

### opt.rs — 最適化戦略

| 関数 | C版 | 内容 |
| --- | --- | --- |
| `optimize_by_joining_curves()` | `OptimizeByJoiningCurves` | 全ステージがカーブのみ → 結合して1つのカーブセットに |
| `optimize_by_resampling()` | `OptimizeByResampling` | 汎用フォールバック: パイプラインをCLUTに再サンプリング |

### pcs.rs — ヘルパー追加

| 関数 | C版 | 内容 |
| --- | --- | --- |
| `reasonable_gridpoints_by_colorspace()` | `_cmsReasonableGridpointsByColorspace` | 色空間のチャネル数とフラグからグリッド点数を決定 |

### xform.rs — フラグ定数追加

| 定数 | C版 | 値 |
| --- | --- | --- |
| `FLAGS_FORCE_CLUT` | `cmsFLAGS_FORCE_CLUT` | 0x0002 |
| `FLAGS_NOWHITEONWHITEFIXUP` | `cmsFLAGS_NOWHITEONWHITEFIXUP` | 0x0004 |
| `FLAGS_CLUT_POST_LINEARIZATION` | `cmsFLAGS_CLUT_POST_LINEARIZATION` | 0x0001 |
| `FLAGS_CLUT_PRE_LINEARIZATION` | `cmsFLAGS_CLUT_PRE_LINEARIZATION` | 0x0010 |
| `FLAGS_HIGHRESPRECALC` | `cmsFLAGS_HIGHRESPRECALC` | 0x0400 |
| `FLAGS_LOWRESPRECALC` | `cmsFLAGS_LOWRESPRECALC` | 0x0800 |

## Deferred

- `OptimizeMatrixShaper` — 8bit固定小数点高速パス（1.14固定小数点行列、256/16385エントリLUT）
- `OptimizeByComputingLinearization` — 前線形化カーブ抽出 + CLUT再サンプリング
- `FixWhiteMisalignment` — 白色点修正（scum dot防止）
- Plugin system（カスタム最適化関数登録）
- 8bit特化評価関数（`PrelinEval8`, `MatShaperEval16`）

## 処理フロー

### optimize_pipeline()

```text
1. FLAGS_NOOPTIMIZE → 即座に return
2. FLAGS_FORCE_CLUT:
   a) pre_optimize()
   b) optimize_by_resampling()
   c) return
3. 通常:
   a) pre_optimize()
   b) optimize_by_joining_curves() → 成功なら return
   c) optimize_by_resampling() → フォールバック
```

### pre_optimize()

```text
1. Identity ステージを全て除去
2. 隣接逆変換ペアを除去:
   - XYZ2Lab + Lab2XYZ
   - Lab2XYZ + XYZ2Lab
   - LabV2toV4 + LabV4toV2
   - LabV4toV2 + LabV2toV4
   - NormFromLabFloat + NormToLabFloat (逆も)
   - NormFromXyzFloat + NormToXyzFloat (逆も)
3. 隣接行列ステージを乗算で結合
```

### optimize_by_joining_curves()

```text
前提条件: 全ステージが CurveSetElem、non-float、16bit
1. パイプラインを4096点で評価してカーブテーブル生成
2. 結果が線形なら Identity に置き換え
3. 非線形なら結合カーブセットで Pipeline を置き換え
```

### optimize_by_resampling()

```text
前提条件: non-float
1. 色空間からグリッド点数を決定
2. CLUT ステージを作成（grid_points^n_in × n_out）
3. 元パイプラインで各グリッド点を評価して CLUT を充填
4. 新パイプライン = CLUT のみ
5. 元パイプラインを新パイプラインで置き換え
```

## 既存モジュール依存

| 依存先 | 利用する関数 |
| --- | --- |
| `pipeline/lut.rs` | `Pipeline`, `Stage::new_clut_16bit_uniform`, `sample_clut_16bit`, `check_and_retrieve_stages` |
| `curves/gamma.rs` | `ToneCurve::build_tabulated_16`, `is_linear`, `join` |
| `curves/intrp.rs` | `quick_saturate_word` |
| `math/pcs.rs` | `reasonable_gridpoints_by_colorspace` (新規) |
| `transform/xform.rs` | フラグ定数 |

## コミット構成（TDD）

### Commit 1: 計画書

### Commit 2 (RED): テスト

- `pre_optimize`: Identity除去、逆ペア除去、行列結合
- `optimize_by_joining_curves`: 2つのガンマカーブ → 結合
- `optimize_by_resampling`: 任意パイプライン → CLUT
- `optimize_pipeline`: FLAGS_NOOPTIMIZE 時はスキップ
- `reasonable_gridpoints_by_colorspace`: RGB=33, CMYK=17

### Commit 3 (GREEN): 実装

ヘルパー追加 → opt.rs 実装 → xform.rs 統合

## 検証方法

```bash
cargo test opt
cargo test xform
cargo test pcs
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt -- --check
```
