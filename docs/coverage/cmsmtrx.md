# cmsmtrx.c カバレッジ

- **C ソース**: `reference/Little-CMS/src/cmsmtrx.c`
- **Rust ファイル**: `src/math/mtrx.rs`
- **概要**: 3次元ベクトル・3x3行列演算

## 公開API

| C 関数               | Rust 対応                | 状態   |
| -------------------- | ------------------------ | ------ |
| `_cmsVEC3init`       | `Vec3::new()`            | 実装済 |
| `_cmsVEC3minus`      | `Sub` trait (`-` 演算子) | 実装済 |
| `_cmsVEC3cross`      | `Vec3::cross()`          | 実装済 |
| `_cmsVEC3dot`        | `Vec3::dot()`            | 実装済 |
| `_cmsVEC3length`     | `Vec3::length()`         | 実装済 |
| `_cmsVEC3distance`   | `Vec3::distance()`       | 実装済 |
| `_cmsMAT3identity`   | `Mat3::identity()`       | 実装済 |
| `_cmsMAT3isIdentity` | `Mat3::is_identity()`    | 実装済 |
| `_cmsMAT3per`        | `Mul` trait (`*` 演算子) | 実装済 |
| `_cmsMAT3inverse`    | `Mat3::inverse()`        | 実装済 |
| `_cmsMAT3solve`      | `Mat3::solve()`          | 実装済 |
| `_cmsMAT3eval`       | `Mat3::eval()`           | 実装済 |

## 備考

- 完全実装。Rustの演算子オーバーロード（`Sub`, `Mul` trait）を活用。
