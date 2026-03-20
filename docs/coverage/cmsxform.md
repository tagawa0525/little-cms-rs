# cmsxform.c カバレッジ

- **C ソース**: `reference/Little-CMS/src/cmsxform.c`
- **Rust ファイル**: `src/transform/xform.rs`
- **概要**: 色変換エンジン（パイプライン実行・並列化）

## 公開API

### Transform作成

| C 関数                                                                 | Rust 対応                       | 状態   |
| ---------------------------------------------------------------------- | ------------------------------- | ------ |
| `cmsCreateTransformTHR` / `cmsCreateTransform`                         | `Transform::new()`              | 実装済 |
| `cmsCreateMultiprofileTransformTHR` / `cmsCreateMultiprofileTransform` | `Transform::new_multiprofile()` | 実装済 |
| `cmsCreateProofingTransformTHR` / `cmsCreateProofingTransform`         | `Transform::new_proofing()`     | 実装済 |
| `cmsCreateExtendedTransform`                                           | `Transform::new_extended()`     | 実装済 |

### Transform実行

| C 関数                     | Rust 対応                   | 状態   |
| -------------------------- | --------------------------- | ------ |
| `cmsDoTransform`           | `Transform::do_transform()` | 実装済 |
| `cmsDoTransformStride`     | —                           | 未実装 |
| `cmsDoTransformLineStride` | —                           | 未実装 |

### Transform管理

| C 関数                        | Rust 対応                            | 状態                |
| ----------------------------- | ------------------------------------ | ------------------- |
| `cmsDeleteTransform`          | `Drop` trait                         | 実装済（暗黙）      |
| `cmsGetTransformContextID`    | —                                    | 未実装（Context系） |
| `cmsGetTransformInputFormat`  | `Transform::input_format()`          | 実装済              |
| `cmsGetTransformOutputFormat` | `Transform::output_format()`         | 実装済              |
| `cmsChangeBuffersFormat`      | `Transform::change_buffers_format()` | 実装済              |
| `cmsTransform2DeviceLink`     | `Transform::to_device_link()`        | 実装済              |

### グローバル状態

| C 関数                     | Rust 対応                             | 状態                   |
| -------------------------- | ------------------------------------- | ---------------------- |
| `cmsSetAdaptationStateTHR` | `Context.adaptation_state` フィールド | 実装済（フィールド）   |
| `cmsSetAdaptationState`    | —                                     | 未実装（グローバル版） |
| `cmsSetAlarmCodesTHR`      | `Context.alarm_codes` フィールド      | 実装済（フィールド）   |
| `cmsGetAlarmCodesTHR`      | `Context.alarm_codes` フィールド      | 実装済（フィールド）   |
| `cmsSetAlarmCodes`         | —                                     | 未実装（グローバル版） |
| `cmsGetAlarmCodes`         | —                                     | 未実装（グローバル版） |

### プラグインAPI（Transform Worker）

| C 関数                            | Rust 対応 | 状態   |
| --------------------------------- | --------- | ------ |
| `_cmsSetTransformUserData`        | —         | 未実装 |
| `_cmsGetTransformUserData`        | —         | 未実装 |
| `_cmsGetTransformFormatters16`    | —         | 未実装 |
| `_cmsGetTransformFormattersFloat` | —         | 未実装 |
| `_cmsGetTransformFlags`           | —         | 未実装 |
| `_cmsGetTransformWorker`          | —         | 未実装 |
| `_cmsGetTransformMaxWorkers`      | —         | 未実装 |
| `_cmsGetTransformWorkerFlags`     | —         | 未実装 |

## 内部関数

| C 関数                          | Rust 対応 | 状態                   |
| ------------------------------- | --------- | ---------------------- |
| `_cmsAllocAdaptationStateChunk` | —         | N/A                    |
| `_cmsAllocAlarmCodesChunk`      | —         | N/A                    |
| `_cmsAllocTransformPluginChunk` | —         | N/A                    |
| `_cmsRegisterTransformPlugin`   | —         | 未実装（プラグイン系） |

## 備考

- Transform作成の中核（`new`, `new_multiprofile`, `new_proofing`, `new_extended`）と実行（`do_transform`）は実装済。
- `cmsDoTransformStride` / `cmsDoTransformLineStride`: ストライド指定の変換。画像のライン単位処理で使用。
- Transform Worker API: C版のプラグインによるカスタム変換ワーカー。並列化やHW加速用。
- グローバル状態関数はContext構造体のフィールドで代替。THRなし版（グローバル）は省略。
