# cmslut.c カバレッジ

- **C ソース**: `reference/Little-CMS/src/cmslut.c`
- **Rust ファイル**: `src/pipeline/lut.rs`
- **概要**: パイプライン・ステージ管理（CLUT、行列、カーブセット）

## 公開API

| C 関数                                  | Rust 対応                               | 状態                             |
| --------------------------------------- | --------------------------------------- | -------------------------------- |
| `_cmsStageAllocPlaceholder`             | —                                       | 未実装                           |
| `cmsStageAllocIdentity`                 | `Stage::new_identity()`                 | 実装済                           |
| `cmsPipelineCheckAndRetreiveStages`     | `Pipeline::check_and_retrieve_stages()` | 実装済                           |
| `cmsStageAllocToneCurves`               | `Stage::new_tone_curves()`              | 実装済                           |
| `_cmsStageAllocIdentityCurves`          | `Stage::new_identity_curves()`          | 実装済                           |
| `cmsStageAllocMatrix`                   | `Stage::new_matrix()`                   | 実装済                           |
| `cmsStageAllocCLut16bitGranular`        | `Stage::new_clut_16bit()`               | 実装済                           |
| `cmsStageAllocCLut16bit`                | `Stage::new_clut_16bit_uniform()`       | 実装済                           |
| `cmsStageAllocCLutFloat`                | `Stage::new_clut_float_uniform()`       | 実装済                           |
| `cmsStageAllocCLutFloatGranular`        | `Stage::new_clut_float()`               | 実装済                           |
| `_cmsStageAllocIdentityCLut`            | `Stage::new_identity_clut()`            | 実装済                           |
| `_cmsQuantizeVal`                       | `quantize_val()`                        | 実装済                           |
| `cmsStageSampleCLut16bit`               | `sample_clut_16bit()`                   | 実装済                           |
| `cmsStageSampleCLutFloat`               | `sample_clut_float()`                   | 実装済                           |
| `cmsSliceSpace16`                       | `slice_space_16()`                      | 実装済                           |
| `cmsSliceSpaceFloat`                    | `slice_space_float()`                   | 実装済                           |
| `_cmsStageAllocLab2XYZ`                 | `Stage::new_lab_to_xyz()`               | 実装済                           |
| `_cmsStageAllocXYZ2Lab`                 | `Stage::new_xyz_to_lab()`               | 実装済                           |
| `_cmsStageAllocLabV2ToV4`               | `Stage::new_lab_v2_to_v4()`             | 実装済                           |
| `_cmsStageAllocLabV4ToV2`               | `Stage::new_lab_v4_to_v2()`             | 実装済                           |
| `cmsStageFree`                          | `Drop` trait                            | 実装済（暗黙）                   |
| `cmsStageInputChannels`                 | `Stage::input_channels()`               | 実装済                           |
| `cmsStageOutputChannels`                | `Stage::output_channels()`              | 実装済                           |
| `cmsStageType`                          | `Stage::stage_type()`                   | 実装済                           |
| `cmsStageData`                          | `Stage::data()`                         | 実装済                           |
| `cmsGetStageContextID`                  | —                                       | 未実装（Context系）              |
| `cmsStageNext`                          | —                                       | 未実装（スライスアクセスで代替） |
| `cmsStageDup`                           | `Clone` trait                           | 実装済（暗黙）                   |
| `cmsPipelineAlloc`                      | `Pipeline::new()`                       | 実装済                           |
| `cmsGetPipelineContextID`               | —                                       | 未実装（Context系）              |
| `cmsPipelineInputChannels`              | `Pipeline::input_channels()`            | 実装済                           |
| `cmsPipelineOutputChannels`             | `Pipeline::output_channels()`           | 実装済                           |
| `cmsPipelineFree`                       | `Drop` trait                            | 実装済（暗黙）                   |
| `cmsPipelineEval16`                     | `Pipeline::eval_16()`                   | 実装済                           |
| `cmsPipelineEvalFloat`                  | `Pipeline::eval_float()`                | 実装済                           |
| `cmsPipelineDup`                        | `Clone` trait                           | 実装済（暗黙）                   |
| `cmsPipelineInsertStage`                | `Pipeline::insert_stage()`              | 実装済                           |
| `cmsPipelineUnlinkStage`                | `Pipeline::remove_stage()`              | 実装済                           |
| `cmsPipelineCat`                        | `Pipeline::cat()`                       | 実装済                           |
| `cmsPipelineSetSaveAs8bitsFlag`         | `Pipeline::set_save_as_8bits()`         | 実装済                           |
| `cmsPipelineGetPtrToFirstStage`         | `Pipeline::first_stage()`               | 実装済                           |
| `cmsPipelineGetPtrToLastStage`          | `Pipeline::last_stage()`                | 実装済                           |
| `cmsPipelineStageCount`                 | `Pipeline::stage_count()`               | 実装済                           |
| `_cmsPipelineSetOptimizationParameters` | —                                       | 未実装（フィールド直接設定）     |
| `cmsPipelineEvalReverseFloat`           | `Pipeline::eval_reverse_float()`        | 実装済                           |

## 内部関数

| C 関数                           | Rust 対応                               | 状態   |
| -------------------------------- | --------------------------------------- | ------ |
| `_cmsStageGetPtrToCurveSet`      | `Stage::curves()`                       | 実装済 |
| `_cmsStageAllocLabV2ToV4curves`  | `Stage::new_lab_v2_to_v4_curves()`      | 実装済 |
| `_cmsStageNormalizeFromLabFloat` | `Stage::new_normalize_from_lab_float()` | 実装済 |
| `_cmsStageNormalizeFromXyzFloat` | `Stage::new_normalize_from_xyz_float()` | 実装済 |
| `_cmsStageNormalizeToLabFloat`   | `Stage::new_normalize_to_lab_float()`   | 実装済 |
| `_cmsStageNormalizeToXyzFloat`   | `Stage::new_normalize_to_xyz_float()`   | 実装済 |
| `_cmsStageClipNegatives`         | `Stage::new_clip_negatives()`           | 実装済 |
| `_cmsStageAllocLabPrelin`        | `Stage::new_lab_prelin()`               | 実装済 |

## 備考

- `cmsGetStageContextID` / `cmsGetPipelineContextID`: RustではContextハンドルを個別オブジェクトに持たない設計差異。
- `cmsStageNext`: C版のリンクドリスト走査。Rustでは `Pipeline::stages()` がスライスを返す。
- `_cmsPipelineSetOptimizationParameters`: Rustではオプティマイザがフィールドを直接設定。
