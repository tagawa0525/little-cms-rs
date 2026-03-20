# cmscam02.c カバレッジ

- **C ソース**: `reference/Little-CMS/src/cmscam02.c`
- **Rust ファイル**: `src/curves/cam02.rs`
- **概要**: CIECAM02色覚モデル

## 公開API

| C 関数               | Rust 対応             | 状態           |
| -------------------- | --------------------- | -------------- |
| `cmsCIECAM02Init`    | `CieCam02::new()`     | 実装済         |
| `cmsCIECAM02Done`    | `Drop` trait          | 実装済（暗黙） |
| `cmsCIECAM02Forward` | `CieCam02::forward()` | 実装済         |
| `cmsCIECAM02Reverse` | `CieCam02::reverse()` | 実装済         |

## 備考

- 完全実装。
