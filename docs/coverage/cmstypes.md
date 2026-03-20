# cmstypes.c カバレッジ

- **C ソース**: `reference/Little-CMS/src/cmstypes.c`
- **Rust ファイル**: `src/profile/tag_types.rs`
- **概要**: ICCタグ型のシリアライズ・デシリアライズ

## インフラストラクチャ

C版はプラグインベースのハンドラ登録システム。Rustでは `match` ベースの直接ディスパッチに置換。

| C 関数                                  | Rust 対応            | 状態     |
| --------------------------------------- | -------------------- | -------- |
| `_cmsRegisterTagTypePlugin`             | `match` ディスパッチ | 設計差異 |
| `_cmsRegisterMultiProcessElementPlugin` | —                    | 未実装   |
| `_cmsGetTagTypeHandler`                 | `match` ディスパッチ | 設計差異 |
| `_cmsAllocTagTypePluginChunk`           | —                    | N/A      |
| `_cmsAllocMPETypePluginChunk`           | —                    | N/A      |
| `_cmsRegisterTagPlugin`                 | —                    | 設計差異 |
| `_cmsAllocTagPluginChunk`               | —                    | N/A      |
| `_cmsGetTagDescriptor`                  | —                    | 設計差異 |
| `_cmsAvoidTypeCheckOnTags`              | —                    | 未実装   |

## タグ型ハンドラ

各タグ型にはC版でRead/Write/Dup/Free関数がある。RustではRead/Writeを実装し、Dup/FreeはClone/Dropで処理。

### 実装済タグ型 (31型)

| タグ型              | C ハンドラ                  | Rust Read                         | Rust Write                         |
| ------------------- | --------------------------- | --------------------------------- | ---------------------------------- |
| XYZ                 | `Type_XYZ_`                 | `read_xyz_type`                   | `write_xyz_type`                   |
| Signature           | `Type_Signature_`           | `read_signature_type`             | `write_signature_type`             |
| DateTime            | `Type_DateTime_`            | `read_datetime_type`              | `write_datetime_type`              |
| S15Fixed16Array     | `Type_S15Fixed16_`          | `read_s15fixed16_type`            | `write_s15fixed16_type`            |
| U16Fixed16Array     | `Type_U16Fixed16_`          | `read_u16fixed16_type`            | `write_u16fixed16_type`            |
| UInt8Array          | `Type_UInt8_`               | `read_uint8_type`                 | `write_uint8_type`                 |
| UInt16Array         | `Type_UInt16_`              | `read_uint16_type`                | `write_uint16_type`                |
| UInt32Array         | `Type_UInt32_`              | `read_uint32_type`                | `write_uint32_type`                |
| UInt64Array         | `Type_UInt64_`              | `read_uint64_type`                | `write_uint64_type`                |
| Text                | `Type_Text_`                | `read_text_type`                  | `write_text_type`                  |
| TextDescription     | `Type_Text_Description_`    | `read_text_description_type`      | `write_text_description_type`      |
| MLU                 | `Type_MLU_`                 | `read_mlu_type`                   | `write_mlu_type`                   |
| Curve               | `Type_Curve_`               | `read_curve_type`                 | `write_curve_type`                 |
| ParametricCurve     | `Type_ParametricCurve_`     | `read_parametric_curve_type`      | `write_parametric_curve_type`      |
| Measurement         | `Type_Measurement_`         | `read_measurement_type`           | `write_measurement_type`           |
| ViewingConditions   | `Type_ViewingConditions_`   | `read_viewing_conditions_type`    | `write_viewing_conditions_type`    |
| Chromaticity        | `Type_Chromaticity_`        | `read_chromaticity_type`          | `write_chromaticity_type`          |
| ColorantOrder       | `Type_ColorantOrderType_`   | `read_colorant_order_type`        | `write_colorant_order_type`        |
| ColorantTable       | `Type_ColorantTable_`       | `read_colorant_table_type`        | `write_colorant_table_type`        |
| NamedColor2         | `Type_NamedColor_`          | `read_named_color_type`           | `write_named_color_type`           |
| Data                | `Type_Data_`                | `read_data_type`                  | `write_data_type`                  |
| Screening           | `Type_Screening_`           | `read_screening_type`             | `write_screening_type`             |
| UcrBg               | `Type_UcrBg_`               | `read_ucr_bg_type`                | `write_ucr_bg_type`                |
| CrdInfo             | `Type_CrdInfo_`             | `read_crd_info_type`              | `write_crd_info_type`              |
| CICP (VideoSignal)  | `Type_VideoSignal_`         | `read_video_signal_type`          | `write_video_signal_type`          |
| ProfileSequenceDesc | `Type_ProfileSequenceDesc_` | `read_profile_sequence_desc_type` | `write_profile_sequence_desc_type` |
| ProfileSequenceId   | `Type_ProfileSequenceId_`   | `read_profile_sequence_id_type`   | `write_profile_sequence_id_type`   |
| vcgt                | `Type_vcgt_`                | `read_vcgt_type`                  | `write_vcgt_type`                  |
| Dictionary          | `Type_Dictionary_`          | `read_dict_type`                  | `write_dict_type`                  |
| Lut8                | `Type_LUT8_`                | `read_lut8_type`                  | `write_lut8_type`                  |
| Lut16               | `Type_LUT16_`               | `read_lut16_type`                 | `write_lut16_type`                 |
| LutAtoB             | `Type_LUTA2B_`              | `read_lut_atob_type`              | `write_lut_atob_type`              |
| LutBtoA             | `Type_LUTB2A_`              | `read_lut_btoa_type`              | `write_lut_btoa_type`              |

### 未実装タグ型

| タグ型                                             | C ハンドラ            | 状態   |
| -------------------------------------------------- | --------------------- | ------ |
| ResponseCurveSet16                                 | `Type_ResponseCurve_` | 未実装 |
| MultiProcessElement                                | `Type_MPE_`           | 未実装 |
| MPEサブ要素 (CurveSet, Matrix, CLUT, BAcs, EAcs等) | `GenericMPEType_*`    | 未実装 |
| MHC2                                               | `Type_MHC2_`          | 未実装 |

## 備考

- C版のプラグインハンドラ登録（`_cmsRegisterTagTypePlugin` 等）はRustの `match` 文で置換。拡張性はtraitベースで実現可能。
- 31のタグ型でRead/Write両方を実装。C版の約160のstatic関数に相当。
- MultiProcessElement (MPE) はFloat版パイプラインの高度な構造。未実装。
