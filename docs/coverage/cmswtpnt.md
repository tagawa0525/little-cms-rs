# cmswtpnt.c カバレッジ

- **C ソース**: `reference/Little-CMS/src/cmswtpnt.c`
- **Rust ファイル**: `src/curves/wtpnt.rs`
- **概要**: 白色点・色順応（Bradford変換）

## 公開API

| C 関数                  | Rust 対応                 | 状態   |
| ----------------------- | ------------------------- | ------ |
| `cmsD50_XYZ`            | `d50_xyz()`               | 実装済 |
| `cmsD50_xyY`            | `d50_xyy()`               | 実装済 |
| `cmsWhitePointFromTemp` | `white_point_from_temp()` | 実装済 |
| `cmsTempFromWhitePoint` | `temp_from_white_point()` | 実装済 |
| `cmsAdaptToIlluminant`  | `adapt_to_illuminant()`   | 実装済 |

## 内部関数

| C 関数                           | Rust 対応                   | 状態   |
| -------------------------------- | --------------------------- | ------ |
| `_cmsAdaptationMatrix`           | `adaptation_matrix()`       | 実装済 |
| `_cmsBuildRGB2XYZtransferMatrix` | `build_rgb_to_xyz_matrix()` | 実装済 |

## 備考

- 完全実装。
