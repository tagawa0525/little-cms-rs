# cmscnvrt.c カバレッジ

- **C ソース**: `reference/Little-CMS/src/cmscnvrt.c`
- **Rust ファイル**: `src/transform/cnvrt.rs`
- **概要**: レンダリングインテント実装・プロファイル間変換パイプライン構築

## 公開API

| C 関数                      | Rust 対応               | 状態   |
| --------------------------- | ----------------------- | ------ |
| `_cmsDefaultICCintents`     | `default_icc_intents()` | 実装済 |
| `cmsGetSupportedIntentsTHR` | —                       | 未実装 |
| `cmsGetSupportedIntents`    | —                       | 未実装 |

## 内部関数

| C 関数                              | Rust 対応         | 状態                   |
| ----------------------------------- | ----------------- | ---------------------- |
| `_cmsLinkProfiles`                  | `link_profiles()` | 実装済                 |
| `_cmsRegisterRenderingIntentPlugin` | —                 | 未実装（プラグイン系） |
| `_cmsAllocIntentsPluginChunk`       | —                 | N/A                    |

## 主要static関数

| C 関数                          | Rust 対応                     | 状態   |
| ------------------------------- | ----------------------------- | ------ |
| `ColorSpaceIsCompatible`        | `color_space_is_compatible()` | 実装済 |
| `IsEmptyLayer`                  | `is_empty_layer()`            | 実装済 |
| `AddConversion`                 | `add_conversion()`            | 実装済 |
| `ComputeAbsoluteIntent`         | `compute_absolute_intent()`   | 実装済 |
| `ComputeBlackPointCompensation` | `compute_bpc()`               | 実装済 |
| `ComputeConversion`             | `compute_conversion()`        | 実装済 |
| `DefaultICCintents`             | `default_icc_intents()` 内    | 実装済 |
| `BlackPreservingKOnlyIntents`   | —                             | 未実装 |
| `BlackPreservingKPlaneIntents`  | —                             | 未実装 |

## 備考

- ICC標準インテント（Perceptual, RelativeColorimetric, Saturation, AbsoluteColorimetric）の変換パイプライン構築は完全実装。
- Black-Preserving系インテント（CMYK用の黒保存変換）は未実装。
- インテント列挙API（`cmsGetSupportedIntents`）は未実装。
