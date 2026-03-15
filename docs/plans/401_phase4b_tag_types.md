# Phase 4b: コアタグ型ハンドラ

**Status**: IMPLEMENTED
**C版ファイル**: `cmstypes.c`（6,252行）のうち簡易型ハンドラ部分
**Rust見積**: ~1,600行（impl）+ ~1,100行（tests）
**ブランチ**: `feat/phase4b-tag-types`

## Context

Phase 4a（IoHandler・Profile・Header・Tag Directory）がマージ済み。Profile は raw タグの読み書きが可能だが、タグのデシリアライズ（cooked read）はまだ実装されていない。

Phase 4 の PR 分割:

- **PR 4a**（完了）: IoHandler + Profile + Header + Tag Directory + 数値ヘルパー
- **PR 4b**（本計画）: TagData enum + 簡易タグ型ハンドラ ~20型 + cooked read/write
- **PR 4c**（別計画）: LUT タグ型（Lut8, Lut16, LutAtoB, LutBtoA, MPE）+ 残りのタグ型 + cmsio1.c

## 変更対象ファイル

| ファイル                   | 操作                                       |
| -------------------------- | ------------------------------------------ |
| `src/types.rs`             | 構造体追加（IccMeasurementConditions 等）  |
| `src/profile/tag_types.rs` | 新規作成                                   |
| `src/profile/io.rs`        | TagDataState 拡張、read_tag/write_tag 追加 |
| `src/profile/mod.rs`       | `pub(crate) mod tag_types;` 追加           |

## 依存する既存 API

| モジュール          | 使用する API                                                                                                           |
| ------------------- | ---------------------------------------------------------------------------------------------------------------------- |
| `profile/io.rs`     | `IoHandler`（数値 I/O ヘルパー全般）, `Profile`, `TagEntry`, `TagDataState`                                            |
| `types.rs`          | `TagTypeSignature`, `TagSignature`, `CieXyz`, `CieXyYTriple`, `DateTimeNumber`, `S15Fixed16`, `U16Fixed16`, `U8Fixed8` |
| `curves/gamma.rs`   | `ToneCurve`（build_parametric, build_tabulated_16, build_gamma）                                                       |
| `pipeline/named.rs` | `Mlu`, `NamedColorList`                                                                                                |
| `context.rs`        | `CmsError`, `ErrorCode`                                                                                                |

## 型定義

### 新規構造体（types.rs に追加）

```rust
/// C版: cmsICCMeasurementConditions
#[derive(Debug, Clone, Default)]
pub struct IccMeasurementConditions {
    pub observer: u32,        // 0=unknown, 1=CIE 1931, 2=CIE 1964
    pub backing: CieXyz,
    pub geometry: u32,        // 0=unknown, 1=0/45 or 45/0, 2=0/d or d/0
    pub flare: f64,
    pub illuminant_type: u32, // D50, D65, etc.
}

/// C版: cmsICCViewingConditions
#[derive(Debug, Clone, Default)]
pub struct IccViewingConditions {
    pub illuminant: CieXyz,
    pub surround: CieXyz,
    pub illuminant_type: u32,
}

/// C版: cmsICCData
#[derive(Debug, Clone)]
pub struct IccData {
    pub flags: u32, // 0=ASCII, 1=binary
    pub data: Vec<u8>,
}

/// C版: cmsScreeningChannel
#[derive(Debug, Clone, Copy, Default)]
pub struct ScreeningChannel {
    pub frequency: f64,
    pub screen_angle: f64,
    pub spot_shape: u32,
}

/// C版: cmsScreening
#[derive(Debug, Clone)]
pub struct Screening {
    pub flags: u32,
    pub channels: Vec<ScreeningChannel>,
}

/// C版: cmsUcrBg
#[derive(Debug, Clone)]
pub struct UcrBg {
    pub ucr: ToneCurve,
    pub bg: ToneCurve,
    pub desc: Mlu,
}
```

### TagData enum（profile/tag_types.rs）

```rust
pub enum TagData {
    Xyz(CieXyz),
    Curve(ToneCurve),
    Mlu(Mlu),
    Signature(u32),
    DateTime(DateTimeNumber),
    Measurement(IccMeasurementConditions),
    ViewingConditions(IccViewingConditions),
    S15Fixed16Array(Vec<f64>),
    U16Fixed16Array(Vec<f64>),
    NamedColor(NamedColorList),
    Chromaticity(CieXyYTriple),
    ColorantOrder(Vec<u8>),
    Data(IccData),
    Screening(Screening),
    UcrBg(UcrBg),
    UInt8Array(Vec<u8>),
    UInt16Array(Vec<u16>),
    UInt32Array(Vec<u32>),
    UInt64Array(Vec<u64>),
    Raw(Vec<u8>),
}
```

### TagDataState 拡張（profile/io.rs）

```rust
pub(crate) enum TagDataState {
    NotLoaded,
    Raw(Vec<u8>),
    Loaded(TagData),  // 新規追加
}
```

## C版→Rust 関数マッピング

### タグ型ハンドラ dispatch

| C版                                    | Rust                                                |
| -------------------------------------- | --------------------------------------------------- |
| `GetHandler` / `_cmsGetTagTypeHandler` | `read_tag_type(io, sig, size) -> Result<TagData>`   |
| `_cmsGetTagDescriptor`                 | `get_tag_descriptor(sig) -> Option<&TagDescriptor>` |
| type decide functions                  | `decide_type(version, data) -> TagTypeSignature`    |

### 簡易型ハンドラ（Read/Write）

| TagTypeSignature      | C版 Read/Write                        | Rust read/write                                                  |
| --------------------- | ------------------------------------- | ---------------------------------------------------------------- |
| Xyz                   | `Type_XYZ_Read/Write`                 | `read_xyz_type` / `write_xyz_type`                               |
| Signature             | `Type_Signature_Read/Write`           | `read_signature_type` / `write_signature_type`                   |
| DateTime              | `Type_DateTime_Read/Write`            | `read_datetime_type` / `write_datetime_type`                     |
| S15Fixed16Array       | `Type_S15Fixed16_Read/Write`          | `read_s15fixed16_type` / `write_s15fixed16_type`                 |
| U16Fixed16Array       | `Type_U16Fixed16_Read/Write`          | `read_u16fixed16_type` / `write_u16fixed16_type`                 |
| UInt8Array            | `Type_UInt8_Read/Write`               | `read_uint8_type` / `write_uint8_type`                           |
| UInt16Array           | `Type_UInt16_Read/Write`              | `read_uint16_type` / `write_uint16_type`                         |
| UInt32Array           | `Type_UInt32_Read/Write`              | `read_uint32_type` / `write_uint32_type`                         |
| UInt64Array           | `Type_UInt64_Read/Write`              | `read_uint64_type` / `write_uint64_type`                         |
| Text                  | `Type_Text_Read/Write`                | `read_text_type` / `write_text_type`                             |
| TextDescription       | `Type_Text_Description_Read/Write`    | `read_text_description_type` / `write_text_description_type`     |
| MultiLocalizedUnicode | `Type_MLU_Read/Write`                 | `read_mlu_type` / `write_mlu_type`                               |
| Curve                 | `Type_Curve_Read/Write`               | `read_curve_type` / `write_curve_type`                           |
| ParametricCurve       | `Type_ParametricCurve_Read/Write`     | `read_parametric_curve_type` / `write_parametric_curve_type`     |
| Measurement           | `Type_Measurement_Read/Write`         | `read_measurement_type` / `write_measurement_type`               |
| ViewingConditions     | `Type_ViewingConditions_Read/Write`   | `read_viewing_conditions_type` / `write_viewing_conditions_type` |
| Chromaticity          | `Type_Chromaticity_Read/Write`        | `read_chromaticity_type` / `write_chromaticity_type`             |
| ColorantOrder         | `Type_ColorantOrderType_Read/Write`   | `read_colorant_order_type` / `write_colorant_order_type`         |
| ColorantTable         | `Type_ColorantTable_Read/Write`       | `read_colorant_table_type` / `write_colorant_table_type`         |
| NamedColor2           | `Type_NamedColor_Read/Write`          | `read_named_color_type` / `write_named_color_type`               |
| Data                  | `Type_Data_Read/Write`                | `read_data_type` / `write_data_type`                             |
| Screening             | `Type_Screening_Read/Write`           | `read_screening_type` / `write_screening_type`                   |
| UcrBg                 | `Type_UcrBg_Read/Write`               | `read_ucr_bg_type` / `write_ucr_bg_type`                         |
| CrdInfo               | `Type_CrdInfo_Read/Write`             | `read_crd_info_type` / `write_crd_info_type`                     |
| ProfileSequenceDesc   | `Type_ProfileSequenceDesc_Read/Write` | `read_profile_seq_desc_type` / `write_profile_seq_desc_type`     |
| ProfileSequenceId     | `Type_ProfileSequenceId_Read/Write`   | `read_profile_seq_id_type` / `write_profile_seq_id_type`         |

### Profile cooked read/write

| C版           | Rust                                         |
| ------------- | -------------------------------------------- |
| `cmsReadTag`  | `profile.read_tag(sig) -> Result<&TagData>`  |
| `cmsWriteTag` | `profile.write_tag(sig, data) -> Result<()>` |

### ユーティリティ

| C版                  | Rust                                         |
| -------------------- | -------------------------------------------- |
| `ReadPositionTable`  | `read_position_table(io, count, reader_fn)`  |
| `WritePositionTable` | `write_position_table(io, count, writer_fn)` |
| `ReadEmbeddedText`   | `read_embedded_text(io, size)`               |

### 4c に延期するもの

- Lut8, Lut16, LutAtoB, LutBtoA（Pipeline 依存）
- MultiProcessElement（Pipeline 依存）
- vcgt（特殊カーブ型）
- Dict, cicp, MHC2
- cmsio1.c パイプライン構築ヘルパー

## コミット構成（TDD）

### Commit 1: RED — 新規構造体テスト

```text
test(types): add IccMeasurementConditions, IccViewingConditions, IccData, Screening tests
```

### Commit 2: GREEN — 新規構造体実装

```text
feat(types): implement IccMeasurementConditions, IccViewingConditions, IccData, Screening, UcrBg
```

### Commit 3: RED — TagData enum + 簡易型テスト

```text
test(tag_types): add TagData enum and XYZ, Signature, DateTime, fixed-point array handler tests
```

### Commit 4: GREEN — TagData enum + 簡易型実装

```text
feat(tag_types): implement TagData enum and XYZ, Signature, DateTime, fixed-point/uint array handlers
```

### Commit 5: RED — Text 系 + Curve 型テスト

```text
test(tag_types): add Text, TextDescription, MLU, Curve, ParametricCurve handler tests
```

### Commit 6: GREEN — Text 系 + Curve 型実装

```text
feat(tag_types): implement Text, TextDescription, MLU, Curve, ParametricCurve handlers
```

### Commit 7: RED — 構造体型ハンドラテスト

```text
test(tag_types): add Measurement, ViewingConditions, Chromaticity, Data, Screening, UcrBg handler tests
```

### Commit 8: GREEN — 構造体型ハンドラ実装

```text
feat(tag_types): implement Measurement, ViewingConditions, Chromaticity, Data, Screening, UcrBg handlers
```

### Commit 9: RED — コレクション型ハンドラテスト

```text
test(tag_types): add ColorantOrder, ColorantTable, NamedColor2, CrdInfo handler tests
```

### Commit 10: GREEN — コレクション型ハンドラ実装

```text
feat(tag_types): implement ColorantOrder, ColorantTable, NamedColor2, CrdInfo handlers
```

### Commit 11: RED — ProfileSequenceDesc/Id + position table テスト

```text
test(tag_types): add ProfileSequenceDesc, ProfileSequenceId, position table tests
```

### Commit 12: GREEN — ProfileSequenceDesc/Id + position table 実装

```text
feat(tag_types): implement ProfileSequenceDesc, ProfileSequenceId and position table helpers
```

### Commit 13: RED — Tag descriptor + cooked read/write 統合テスト

```text
test(io): add Profile.read_tag()/write_tag() cooked read/write integration tests
```

### Commit 14: GREEN — Tag descriptor + cooked read/write 統合実装

```text
feat(io): implement tag descriptor table, Profile.read_tag()/write_tag()
```

## エッジケース・エラー処理

- **未知の TagTypeSignature**: `TagData::Raw(Vec<u8>)` にフォールバック
- **TextDescription**: Unicode/ScriptCode セクション欠落を許容（C版互換）
- **Curve count=0**: linear（gamma=1.0）として扱う
- **Curve count=1**: 8.8 固定小数点ガンマ値
- **MLU record length != 12**: エラー
- **ColorantOrder**: MAX_CHANNELS(16) でキャップ
- **NamedColor2 device coords > MAX_CHANNELS**: エラー
- **バージョン依存型選択（write）**: v2 → TextDescription/Curve、v4 → MLU/ParametricCurve
- **壊れたベンダー型**: CorbisBrokenXYZ (0x17A505B8)、MonacoBrokenCurve (0x9478ee00) を特殊処理

## 検証方法

```bash
cargo test tag_types          # tag_types モジュールテスト
cargo test profile::io        # io 統合テスト
cargo test                    # 全テスト（回帰確認）
cargo clippy --all-targets -- -D warnings
cargo fmt -- --check
```
