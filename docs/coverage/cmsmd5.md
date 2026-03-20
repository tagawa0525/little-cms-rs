# cmsmd5.c カバレッジ

- **C ソース**: `reference/Little-CMS/src/cmsmd5.c`
- **Rust ファイル**: `src/math/md5.rs`
- **概要**: MD5ハッシュ・プロファイルID計算

## 公開API

| C 関数            | Rust 対応                   | 状態   |
| ----------------- | --------------------------- | ------ |
| `cmsMD5alloc`     | `Md5::new()`                | 実装済 |
| `cmsMD5add`       | `Md5::update()`             | 実装済 |
| `cmsMD5finish`    | `Md5::finish()`             | 実装済 |
| `cmsMD5computeID` | `Profile::compute_md5_id()` | 実装済 |

## 備考

- `cmsMD5computeID`は`Profile::compute_md5_id()`として`src/profile/io.rs`に実装（プロファイルメソッド）。
- `Md5::digest()`はnew+update+finishの便利メソッド。
