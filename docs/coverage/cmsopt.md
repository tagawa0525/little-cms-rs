# cmsopt.c カバレッジ

- **C ソース**: `reference/Little-CMS/src/cmsopt.c`
- **Rust ファイル**: `src/transform/opt.rs`
- **概要**: パイプライン最適化（カーブ結合、行列畳み込み、リサンプリング等）

## 公開API

| C 関数                 | Rust 対応             | 状態   |
| ---------------------- | --------------------- | ------ |
| `_cmsOptimizePipeline` | `optimize_pipeline()` | 実装済 |

## 内部関数

| C 関数                             | Rust 対応 | 状態                   |
| ---------------------------------- | --------- | ---------------------- |
| `_cmsAllocOptimizationPluginChunk` | —         | N/A                    |
| `_cmsRegisterOptimizationPlugin`   | —         | 未実装（プラグイン系） |

## 最適化戦略（static関数）

| C 関数                             | Rust 対応                               | 状態   |
| ---------------------------------- | --------------------------------------- | ------ |
| `PreOptimize`                      | `pre_optimize()`                        | 実装済 |
| `OptimizeByResampling`             | `optimize_by_resampling()`              | 実装済 |
| `OptimizeByComputingLinearization` | `optimize_by_computing_linearization()` | 実装済 |
| `OptimizeByJoiningCurves`          | `optimize_by_joining_curves()`          | 実装済 |
| `OptimizeMatrixShaper`             | `optimize_by_matrix_shaper()`           | 実装済 |
| `PrelinOpt16alloc`                 | `optimize_by_resampling()` 内           | 実装済 |
| `PrelinOpt8alloc`                  | `prelin8_alloc()`                       | 実装済 |
| `FixWhiteMisalignment`             | `fix_white_misalignment()`              | 実装済 |
| `SlopeLimiting`                    | `slope_limiting()`                      | 実装済 |
| `FillFirstShaper`                  | `fill_first_shaper()`                   | 実装済 |
| `FillSecondShaper`                 | `fill_second_shaper()`                  | 実装済 |

## 備考

- 4つの主要最適化戦略（リサンプリング、線形化、カーブ結合、行列シェイパー）全て実装済。
- 8bit/16bit高速パスも実装済。
- プラグインによるカスタム最適化の登録のみ未実装。
