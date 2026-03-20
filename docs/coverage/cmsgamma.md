# cmsgamma.c カバレッジ

- **C ソース**: `reference/Little-CMS/src/cmsgamma.c`
- **Rust ファイル**: `src/curves/gamma.rs`
- **概要**: トーンカーブ（パラメトリック・テーブル・セグメント）

## 公開API

| C 関数                                 | Rust 対応                            | 状態           |
| -------------------------------------- | ------------------------------------ | -------------- |
| `cmsBuildTabulatedToneCurve16`         | `ToneCurve::build_tabulated_16()`    | 実装済         |
| `cmsBuildSegmentedToneCurve`           | `ToneCurve::build_segmented()`       | 実装済         |
| `cmsBuildTabulatedToneCurveFloat`      | `ToneCurve::build_tabulated_float()` | 実装済         |
| `cmsBuildParametricToneCurve`          | `ToneCurve::build_parametric()`      | 実装済         |
| `cmsBuildGamma`                        | `ToneCurve::build_gamma()`           | 実装済         |
| `cmsFreeToneCurve`                     | `Drop` trait                         | 実装済（暗黙） |
| `cmsFreeToneCurveTriple`               | `Drop` trait                         | 実装済（暗黙） |
| `cmsDupToneCurve`                      | `Clone` trait                        | 実装済（暗黙） |
| `cmsJoinToneCurve`                     | `ToneCurve::join()`                  | 実装済         |
| `cmsReverseToneCurveEx`                | `ToneCurve::reverse_with_samples()`  | 実装済         |
| `cmsReverseToneCurve`                  | `ToneCurve::reverse()`               | 実装済         |
| `cmsSmoothToneCurve`                   | `ToneCurve::smooth()`                | 実装済         |
| `cmsIsToneCurveLinear`                 | `ToneCurve::is_linear()`             | 実装済         |
| `cmsIsToneCurveMonotonic`              | `ToneCurve::is_monotonic()`          | 実装済         |
| `cmsIsToneCurveDescending`             | `ToneCurve::is_descending()`         | 実装済         |
| `cmsIsToneCurveMultisegment`           | `ToneCurve::is_multisegment()`       | 実装済         |
| `cmsGetToneCurveParametricType`        | `ToneCurve::parametric_type()`       | 実装済         |
| `cmsEvalToneCurveFloat`                | `ToneCurve::eval_f32()`              | 実装済         |
| `cmsEvalToneCurve16`                   | `ToneCurve::eval_u16()`              | 実装済         |
| `cmsEstimateGamma`                     | `ToneCurve::estimate_gamma()`        | 実装済         |
| `cmsGetToneCurveEstimatedTableEntries` | `ToneCurve::table16_len()`           | 実装済         |
| `cmsGetToneCurveEstimatedTable`        | `ToneCurve::table16()`               | 実装済         |
| `cmsGetToneCurveSegment`               | `ToneCurve::segment()`               | 実装済         |

## 内部関数

| C 関数                               | Rust 対応 | 状態                |
| ------------------------------------ | --------- | ------------------- |
| `_cmsAllocCurvesPluginChunk`         | —         | N/A（プラグイン系） |
| `_cmsRegisterParametricCurvesPlugin` | —         | N/A（プラグイン系） |

## 備考

- 完全実装。全23の公開API関数が対応するRustメソッドを持つ。
- C版のメモリ管理（Free/Dup）はRustの`Drop`/`Clone`で自動処理。
- プラグイン登録はRustでは直接ディスパッチに置換。
