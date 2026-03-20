# cmsintrp.c カバレッジ

- **C ソース**: `reference/Little-CMS/src/cmsintrp.c`
- **Rust ファイル**: `src/curves/intrp.rs`
- **概要**: 多次元LUT補間（1D線形、2Dバイリニア、3Dトリリニア/テトラヘドラル、4-15D）

## 公開API

| C 関数                    | Rust 対応                         | 状態           |
| ------------------------- | --------------------------------- | -------------- |
| `_cmsComputeInterpParams` | `InterpParams::compute_uniform()` | 実装済         |
| `_cmsFreeInterpParams`    | `Drop` trait                      | 実装済（暗黙） |

## 内部関数

| C 関数                        | Rust 対応                                     | 状態                |
| ----------------------------- | --------------------------------------------- | ------------------- |
| `_cmsComputeInterpParamsEx`   | `InterpParams::compute()`                     | 実装済              |
| `_cmsSetInterpolationRoutine` | `eval_16()` / `eval_float()` 内でディスパッチ | 実装済（統合）      |
| `_cmsAllocInterpPluginChunk`  | —                                             | N/A（プラグイン系） |
| `_cmsRegisterInterpPlugin`    | —                                             | N/A（プラグイン系） |

## 補間アルゴリズム（static関数）

| C 関数                             | Rust 対応                                | 状態   |
| ---------------------------------- | ---------------------------------------- | ------ |
| `LinLerp1D`                        | `lin_lerp_1d()`                          | 実装済 |
| `LinLerp1Dfloat`                   | `lin_lerp_1d_float()`                    | 実装済 |
| `Eval1Input`                       | `eval_1_input()`                         | 実装済 |
| `Eval1InputFloat`                  | `eval_1_input_float()`                   | 実装済 |
| `BilinearInterp16`                 | `bilinear_interp_16()`                   | 実装済 |
| `BilinearInterpFloat`              | `bilinear_interp_float()`                | 実装済 |
| `TrilinearInterp16`                | `trilinear_interp_16()`                  | 実装済 |
| `TrilinearInterpFloat`             | `trilinear_interp_float()`               | 実装済 |
| `TetrahedralInterp16`              | `tetrahedral_interp_16()`                | 実装済 |
| `TetrahedralInterpFloat`           | `tetrahedral_interp_float()`             | 実装済 |
| `Eval4Inputs`                      | `eval_n_inputs_16()`                     | 実装済 |
| `Eval4InputsFloat`                 | `eval_n_inputs_float()`                  | 実装済 |
| `Eval5-15Inputs` (マクロ生成)      | `eval_n_inputs_16()` (再帰的統一実装)    | 実装済 |
| `Eval5-15InputsFloat` (マクロ生成) | `eval_n_inputs_float()` (再帰的統一実装) | 実装済 |

## 備考

- 完全実装。C版のマクロ生成による`Eval5-15Inputs`/`Eval5-15InputsFloat`（22関数）はRustでは再帰的な統一関数で実装。
