# cmsio1.c カバレッジ

- **C ソース**: `reference/Little-CMS/src/cmsio1.c`
- **Rust ファイル**: `src/profile/io.rs`
- **概要**: タグ読み書き・プロファイルヘッダ処理・LUTパイプライン構築

## 公開API

| C 関数                   | Rust 対応                           | 状態                                |
| ------------------------ | ----------------------------------- | ----------------------------------- |
| `_cmsReadInputLUT`       | `Profile::read_input_lut()`         | 実装済                              |
| `_cmsReadOutputLUT`      | `Profile::read_output_lut()`        | 実装済                              |
| `_cmsReadDevicelinkLUT`  | `Profile::read_devicelink_lut()`    | 実装済                              |
| `cmsIsMatrixShaper`      | `Profile::is_matrix_shaper()`       | 実装済                              |
| `cmsIsCLUT`              | `Profile::is_clut()`                | 実装済                              |
| `cmsIsIntentSupported`   | `Profile::is_intent_supported()`    | 実装済                              |
| `cmsGetProfileInfo`      | —                                   | 対象外（wchar_t API、Rustでは不要） |
| `cmsGetProfileInfoASCII` | `Profile::get_profile_info_ascii()` | 実装済                              |
| `cmsGetProfileInfoUTF8`  | `Profile::get_profile_info_utf8()`  | 実装済                              |

## 内部関数

| C 関数                       | Rust 対応                           | 状態   |
| ---------------------------- | ----------------------------------- | ------ |
| `_cmsReadMediaWhitePoint`    | `Profile::read_media_white_point()` | 実装済 |
| `_cmsReadCHAD`               | `Profile::read_chad()`              | 実装済 |
| `_cmsReadProfileSequence`    | —                                   | 未実装 |
| `_cmsWriteProfileSequence`   | —                                   | 未実装 |
| `_cmsCompileProfileSequence` | —                                   | 未実装 |

## 備考

- LUTパイプライン構築の中核機能（Input/Output/DeviceLink LUT読み込み）は完全実装。
- `cmsGetProfileInfo`（wchar_t版）はRustでは不要。`get_profile_info_ascii()` / `get_profile_info_utf8()` で代替。
- ProfileSequence関連: `_cmsReadProfileSequence` / `_cmsWriteProfileSequence` / `_cmsCompileProfileSequence` は未実装。
