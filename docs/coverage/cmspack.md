# cmspack.c カバレッジ

- **C ソース**: `reference/Little-CMS/src/cmspack.c`
- **Rust ファイル**: `src/pipeline/pack.rs`
- **概要**: ピクセルフォーマッタ（8/16/32bit、planar/chunky変換）

## 公開API

| C 関数                               | Rust 対応                                      | 状態                   |
| ------------------------------------ | ---------------------------------------------- | ---------------------- |
| `_cmsGetFormatter`                   | `find_formatter_in()` / `find_formatter_out()` | 実装済                 |
| `cmsFormatterForColorspaceOfProfile` | `formatter_for_colorspace()` (samp.rs)         | 実装済（別モジュール） |
| `cmsFormatterForPCSOfProfile`        | —                                              | 未実装                 |

## 内部関数

| C 関数                           | Rust 対応 | 状態                                       |
| -------------------------------- | --------- | ------------------------------------------ |
| `_cmsAllocFormattersPluginChunk` | —         | N/A（プラグイン系）                        |
| `_cmsRegisterFormattersPlugin`   | —         | N/A（プラグイン系）                        |
| `_cmsFormatterIsFloat`           | —         | 未実装（`PixelFormat::is_float()` で代替） |
| `_cmsFormatterIs8bit`            | —         | 未実装（`PixelFormat::bytes()` で代替）    |

## フォーマッタ実装状況

C版には約100のstatic Unroll/Pack関数がある。Rust側の対応:

### Unpack（入力フォーマッタ）

- Chunky 8bit / 16bit: 実装済
- Planar 8bit / 16bit: 実装済
- Float / Double: 実装済
- Half-float: 実装済
- Lab V2 (8bit / 16bit): 実装済
- Lab/XYZ float正規化: 実装済
- Premultiplied alpha: 実装済

### Pack（出力フォーマッタ）

- Chunky 8bit / 16bit: 実装済
- Planar 8bit / 16bit: 実装済
- Float / Double: 実装済
- Half-float: 実装済
- Lab V2: 実装済
- Lab/XYZ float: 実装済

## 備考

- フォーマッタの基盤と主要なバリアントは全て実装済。
- `_cmsFormatterIsFloat` / `_cmsFormatterIs8bit` はRustでは `PixelFormat` のメソッドとして統合。
