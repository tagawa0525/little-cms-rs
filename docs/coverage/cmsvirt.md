# cmsvirt.c カバレッジ

- **C ソース**: `reference/Little-CMS/src/cmsvirt.c`
- **Rust ファイル**: `src/profile/virt.rs`
- **概要**: 仮想プロファイル生成（sRGB、Lab、リンクプロファイル等）

## 公開API

| C 関数                                                                     | Rust 対応                                  | 状態   |
| -------------------------------------------------------------------------- | ------------------------------------------ | ------ |
| `cmsCreateRGBProfileTHR` / `cmsCreateRGBProfile`                           | `Profile::new_rgb()`                       | 実装済 |
| `cmsCreateGrayProfileTHR` / `cmsCreateGrayProfile`                         | `Profile::new_gray()`                      | 実装済 |
| `cmsCreateLinearizationDeviceLinkTHR` / `cmsCreateLinearizationDeviceLink` | `Profile::new_linearization_device_link()` | 実装済 |
| `cmsCreateInkLimitingDeviceLinkTHR` / `cmsCreateInkLimitingDeviceLink`     | `Profile::new_ink_limiting_device_link()`  | 実装済 |
| `cmsCreateLab2ProfileTHR` / `cmsCreateLab2Profile`                         | `Profile::new_lab2()`                      | 実装済 |
| `cmsCreateLab4ProfileTHR` / `cmsCreateLab4Profile`                         | `Profile::new_lab4()`                      | 実装済 |
| `cmsCreateXYZProfileTHR` / `cmsCreateXYZProfile`                           | `Profile::new_xyz()`                       | 実装済 |
| `cmsCreate_sRGBProfileTHR` / `cmsCreate_sRGBProfile`                       | `Profile::new_srgb()`                      | 実装済 |
| `cmsCreate_OkLabProfile`                                                   | `Profile::new_oklab()`                     | 実装済 |
| `cmsCreateBCHSWabstractProfileTHR` / `cmsCreateBCHSWabstractProfile`       | `Profile::new_bchsw_abstract()`            | 実装済 |
| `cmsCreateNULLProfileTHR` / `cmsCreateNULLProfile`                         | `Profile::new_null()`                      | 実装済 |
| `cmsTransform2DeviceLink`                                                  | `Transform::to_device_link()` (xform.rs)   | 実装済 |

## 備考

- 完全実装。C版のTHR/非THRペアはRustでは単一のメソッドに統合（Contextは引数で渡す設計）。
