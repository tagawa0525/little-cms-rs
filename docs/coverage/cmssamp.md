# cmssamp.c カバレッジ

- **C ソース**: `reference/Little-CMS/src/cmssamp.c`
- **Rust ファイル**: `src/transform/samp.rs`
- **概要**: ブラックポイント検出

## 公開API

| C 関数                           | Rust 対応                   | 状態             |
| -------------------------------- | --------------------------- | ---------------- |
| `cmsDetectBlackPoint`            | `detect_black_point()`      | 実装済           |
| `cmsDetectDestinationBlackPoint` | `detect_dest_black_point()` | 実装済（簡略版） |

## 備考

- `cmsDetectDestinationBlackPoint` は簡略化実装。Adobe L*ランプアルゴリズムの完全版は未実装で、`detect_black_point()` に委譲。
