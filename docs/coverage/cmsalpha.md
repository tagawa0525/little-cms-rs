# cmsalpha.c カバレッジ

- **C ソース**: `reference/Little-CMS/src/cmsalpha.c`
- **Rust ファイル**: `src/transform/alpha.rs`
- **概要**: アルファチャネル処理

## 内部関数

| C 関数                    | Rust 対応                 | 状態   |
| ------------------------- | ------------------------- | ------ |
| `_cmsHandleExtraChannels` | `handle_extra_channels()` | 実装済 |

## 備考

- 完全実装。各ビット深度間の変換（8/16/float/double/half）を含むエクストラチャネルコピー機能。
