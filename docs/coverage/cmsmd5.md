# cmsmd5.c カバレッジ

- **C ソース**: `reference/Little-CMS/src/cmsmd5.c`
- **Rust ファイル**: `src/math/md5.rs`
- **概要**: MD5ハッシュ・プロファイルID計算

## 公開API

| C 関数            | Rust 対応       | 状態   |
| ----------------- | --------------- | ------ |
| `cmsMD5alloc`     | `Md5::new()`    | 実装済 |
| `cmsMD5add`       | `Md5::update()` | 実装済 |
| `cmsMD5finish`    | `Md5::finish()` | 実装済 |
| `cmsMD5computeID` | —               | 未実装 |

## 備考

- `cmsMD5computeID`はICCプロファイル全体をハッシュしてProfileIDを設定する高レベル関数。MD5コア自体は完全実装。
- `Md5::digest()`はnew+update+finishの便利メソッド。
