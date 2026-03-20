# cmshalf.c カバレッジ

- **C ソース**: `reference/Little-CMS/src/cmshalf.c`
- **Rust ファイル**: `src/math/half.rs`
- **概要**: 半精度浮動小数点（IEEE 754-2008）変換

## 公開API

| C 関数           | Rust 対応         | 状態   |
| ---------------- | ----------------- | ------ |
| `_cmsHalf2Float` | `half_to_float()` | 実装済 |
| `_cmsFloat2Half` | `float_to_half()` | 実装済 |

## 備考

- 完全実装。C版と同じルックアップテーブル方式。
