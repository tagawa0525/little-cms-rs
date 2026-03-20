# cmssm.c カバレッジ

- **C ソース**: `reference/Little-CMS/src/cmssm.c`
- **Rust ファイル**: `src/transform/sm.rs`
- **概要**: ガマットバウンダリ記述

## 公開API

| C 関数             | Rust 対応                    | 状態           |
| ------------------ | ---------------------------- | -------------- |
| `cmsGBDAlloc`      | `GamutBoundary::new()`       | 実装済         |
| `cmsGBDFree`       | `Drop` trait                 | 実装済（暗黙） |
| `cmsGDBAddPoint`   | `GamutBoundary::add_point()` | 実装済         |
| `cmsGDBCheckPoint` | `GamutBoundary::check()`     | 実装済         |
| `cmsGDBCompute`    | `GamutBoundary::compute()`   | 実装済         |

## 備考

- 完全実装。
