# cmsio1.c カバレッジ

- **C ソース**: `reference/Little-CMS/src/cmsio1.c`
- **Rust ファイル**: `src/profile/io.rs`
- **概要**: タグ読み書き・プロファイルヘッダ処理・LUTパイプライン構築

## 公開API

| C 関数                   | Rust 対応                        | 状態   |
| ------------------------ | -------------------------------- | ------ |
| `_cmsReadInputLUT`       | `Profile::read_input_lut()`      | 実装済 |
| `_cmsReadOutputLUT`      | `Profile::read_output_lut()`     | 実装済 |
| `_cmsReadDevicelinkLUT`  | `Profile::read_devicelink_lut()` | 実装済 |
| `cmsIsMatrixShaper`      | `Profile::is_matrix_shaper()`    | 実装済 |
| `cmsIsCLUT`              | `Profile::is_clut()`             | 実装済 |
| `cmsIsIntentSupported`   | `Profile::is_intent_supported()` | 実装済 |
| `cmsGetProfileInfo`      | —                                | 未実装 |
| `cmsGetProfileInfoASCII` | —                                | 未実装 |
| `cmsGetProfileInfoUTF8`  | —                                | 未実装 |

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
- `cmsGetProfileInfo*`: プロファイルのMLU情報取得関数群。直接MLUタグを読んで取得可能だが、便利関数として未実装。
- ProfileSequence関連: `_cmsReadProfileSequence` / `_cmsWriteProfileSequence` / `_cmsCompileProfileSequence` は未実装。
