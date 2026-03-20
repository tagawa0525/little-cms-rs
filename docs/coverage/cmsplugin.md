# cmsplugin.c カバレッジ

- **C ソース**: `reference/Little-CMS/src/cmsplugin.c`
- **Rust ファイル**: `src/profile/io.rs`, `src/types.rs`, `src/context.rs`
- **概要**: I/Oプリミティブ・プラグインレジストリ・コンテキスト管理

## I/O読み書きプリミティブ

| C 関数                     | Rust 対応                       | 状態   |
| -------------------------- | ------------------------------- | ------ |
| `_cmsReadUInt8Number`      | `IoHandler::read_u8()`          | 実装済 |
| `_cmsReadUInt16Number`     | `IoHandler::read_u16()`         | 実装済 |
| `_cmsReadUInt16Array`      | `IoHandler::read_u16_array()`   | 実装済 |
| `_cmsReadUInt32Number`     | `IoHandler::read_u32()`         | 実装済 |
| `_cmsReadFloat32Number`    | `IoHandler::read_f32()`         | 実装済 |
| `_cmsReadUInt64Number`     | `IoHandler::read_u64()`         | 実装済 |
| `_cmsRead15Fixed16Number`  | `IoHandler::read_s15fixed16()`  | 実装済 |
| `_cmsReadXYZNumber`        | `IoHandler::read_xyz()`         | 実装済 |
| `_cmsWriteUInt8Number`     | `IoHandler::write_u8()`         | 実装済 |
| `_cmsWriteUInt16Number`    | `IoHandler::write_u16()`        | 実装済 |
| `_cmsWriteUInt16Array`     | `IoHandler::write_u16_array()`  | 実装済 |
| `_cmsWriteUInt32Number`    | `IoHandler::write_u32()`        | 実装済 |
| `_cmsWriteFloat32Number`   | `IoHandler::write_f32()`        | 実装済 |
| `_cmsWriteUInt64Number`    | `IoHandler::write_u64()`        | 実装済 |
| `_cmsWrite15Fixed16Number` | `IoHandler::write_s15fixed16()` | 実装済 |
| `_cmsWriteXYZNumber`       | `IoHandler::write_xyz()`        | 実装済 |

## 固定小数点変換

| C 関数                  | Rust 対応                       | 状態   |
| ----------------------- | ------------------------------- | ------ |
| `_cms8Fixed8toDouble`   | `impl From<U8Fixed8> for f64`   | 実装済 |
| `_cmsDoubleTo8Fixed8`   | `impl From<f64> for U8Fixed8`   | 実装済 |
| `_cms15Fixed16toDouble` | `impl From<S15Fixed16> for f64` | 実装済 |
| `_cmsDoubleTo15Fixed16` | `impl From<f64> for S15Fixed16` | 実装済 |

## 日時・型ベース・アライメント

| C 関数                     | Rust 対応                           | 状態   |
| -------------------------- | ----------------------------------- | ------ |
| `_cmsDecodeDateTimeNumber` | `read_header()` 内でインライン処理  | 実装済 |
| `_cmsEncodeDateTimeNumber` | `write_header()` 内でインライン処理 | 実装済 |
| `_cmsReadTypeBase`         | `IoHandler::read_type_base()`       | 実装済 |
| `_cmsWriteTypeBase`        | `IoHandler::write_type_base()`      | 実装済 |
| `_cmsReadAlignment`        | `IoHandler::read_alignment()`       | 実装済 |
| `_cmsWriteAlignment`       | `IoHandler::write_alignment()`      | 実装済 |

## エンディアン変換

| C 関数                  | Rust 対応              | 状態                |
| ----------------------- | ---------------------- | ------------------- |
| `_cmsAdjustEndianess16` | `u16::from_be_bytes()` | N/A（Rust標準機能） |
| `_cmsAdjustEndianess32` | `u32::from_be_bytes()` | N/A（Rust標準機能） |
| `_cmsAdjustEndianess64` | `u64::from_be_bytes()` | N/A（Rust標準機能） |

## プラグイン・コンテキスト管理

| C 関数                                             | Rust 対応        | 状態           |
| -------------------------------------------------- | ---------------- | -------------- |
| `_cmsIOPrintf`                                     | —                | 未実装         |
| `cmsPlugin` / `cmsPluginTHR`                       | —                | 未実装         |
| `cmsUnregisterPlugins` / `cmsUnregisterPluginsTHR` | —                | 未実装         |
| `cmsCreateContext`                                 | `Context::new()` | 部分的実装     |
| `cmsDupContext`                                    | —                | 未実装         |
| `cmsDeleteContext`                                 | `Drop` trait     | 実装済（暗黙） |
| `cmsGetContextUserData`                            | —                | 未実装         |

## 内部関数

| C 関数                      | Rust 対応 | 状態   |
| --------------------------- | --------- | ------ |
| `_cmsPluginMalloc`          | —         | N/A    |
| `_cmsGetContext`            | —         | N/A    |
| `_cmsContextGetClientChunk` | —         | 未実装 |
| `_cmsGetTime`               | —         | 未実装 |

## 備考

- I/Oプリミティブ（数値読み書き、固定小数点変換）は完全実装。
- プラグインシステム全体は未実装。Rustでは静的ディスパッチで代替。
- `cmsCreateContext` は基本的なContextのみ。ユーザーデータやプラグインの受け渡しは未対応。
