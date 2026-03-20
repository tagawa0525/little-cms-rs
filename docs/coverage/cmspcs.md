# cmspcs.c カバレッジ

- **C ソース**: `reference/Little-CMS/src/cmspcs.c`
- **Rust ファイル**: `src/math/pcs.rs`, `src/types.rs`
- **概要**: PCS色空間変換（XYZ↔Lab、エンコード/デコード）、DeltaE計算

## 公開API

| C 関数                                 | Rust 対応                                | 状態                                    |
| -------------------------------------- | ---------------------------------------- | --------------------------------------- |
| `cmsXYZ2xyY`                           | `xyz_to_xyy()`                           | 実装済                                  |
| `cmsxyY2XYZ`                           | `xyy_to_xyz()`                           | 実装済                                  |
| `cmsXYZ2Lab`                           | `xyz_to_lab()`                           | 実装済                                  |
| `cmsLab2XYZ`                           | `lab_to_xyz()`                           | 実装済                                  |
| `cmsLabEncoded2FloatV2`                | —                                        | 未実装                                  |
| `cmsLabEncoded2Float`                  | `pcs_encoded_lab_to_float()`             | 実装済                                  |
| `cmsFloat2LabEncodedV2`                | —                                        | 未実装                                  |
| `cmsFloat2LabEncoded`                  | `float_to_pcs_encoded_lab()`             | 実装済                                  |
| `cmsLab2LCh`                           | `lab_to_lch()`                           | 実装済                                  |
| `cmsLCh2Lab`                           | `lch_to_lab()`                           | 実装済                                  |
| `cmsFloat2XYZEncoded`                  | `float_to_pcs_encoded_xyz()`             | 実装済                                  |
| `cmsXYZEncoded2Float`                  | `pcs_encoded_xyz_to_float()`             | 実装済                                  |
| `cmsDeltaE`                            | `delta_e()`                              | 実装済                                  |
| `cmsCIE94DeltaE`                       | `delta_e_cie94()`                        | 実装済                                  |
| `cmsBFDdeltaE`                         | `delta_e_bfd()`                          | 実装済                                  |
| `cmsCMCdeltaE`                         | `delta_e_cmc()`                          | 実装済                                  |
| `cmsCIE2000DeltaE`                     | `delta_e_ciede2000()`                    | 実装済                                  |
| `_cmsReasonableGridpointsByColorspace` | `reasonable_gridpoints()`                | 実装済                                  |
| `_cmsICCcolorSpace`                    | `ColorSpaceSignature::from_pixel_type()` | 実装済                                  |
| `_cmsLCMScolorSpace`                   | `ColorSpaceSignature::to_pixel_type()`   | 実装済                                  |
| `cmsChannelsOfColorSpace`              | `ColorSpaceSignature::channels()`        | 実装済                                  |
| `cmsChannelsOf`                        | —                                        | 未実装（`channels()` メソッドで代替可） |

## 内部関数

| C 関数                 | Rust 対応              | 状態   |
| ---------------------- | ---------------------- | ------ |
| `_cmsEndPointsBySpace` | `endpoints_by_space()` | 実装済 |

## 備考

- V2固有の16bitエンコード/デコード（`cmsLabEncoded2FloatV2`, `cmsFloat2LabEncodedV2`）は未実装。V2→V4変換は `pipeline/pack.rs` の `lab_v2_to_v4` / `lab_v4_to_v2` で対応。
- `cmsChannelsOf` は `cmsChannelsOfColorSpace` の単なるラッパーで、Rustではメソッド形式（`ColorSpaceSignature::channels()`）で統一。
