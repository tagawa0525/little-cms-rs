# Phase 4a: Profile I/O 基盤（IoHandler・Profile・Header・Tag Directory）

**Status**: IMPLEMENTED
**C版ファイル**: `cmsio0.c`（2,151行）+ `cmsplugin.c`（number helpers 部分、約320行）
**Rust見積**: ~1,400行（impl）+ ~700行（tests）
**ブランチ**: `feat/phase4-io`

## Context

Phase 3b（Pipeline/Stage）がマージ済み。次は ICC プロファイルの読み書き基盤を実装する。

Profile I/O は Phase 4 全体で 3 つの PR に分割する：

- **PR 4a**（本計画）: IoHandler + Profile 構造体 + ヘッダー + タグディレクトリ + 数値 I/O ヘルパー
- **PR 4b**（別計画）: コアタグ型ハンドラ（XYZ, Curve, MLU 等 ~20 型）
- **PR 4c**（別計画）: LUT タグ型 + 残りのタグ型 + cmsio1.c パイプライン構築ヘルパー

本 PR は Profile の骨格を確立し、raw タグの読み書きまでをサポートする。タグのデシリアライズ（cooked read）は PR 4b で実装する。

## 変更対象ファイル

| ファイル             | 操作                    |
| -------------------- | ----------------------- |
| `src/profile/mod.rs` | 新規作成                |
| `src/profile/io.rs`  | 新規作成                |
| `src/lib.rs`         | `pub mod profile;` 追加 |

## 依存する既存API

| モジュール   | 使用するAPI                                                                                                                                                                                                         |
| ------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `types.rs`   | `IccHeader`, `TagSignature`, `TagTypeSignature`, `ColorSpaceSignature`, `ProfileClassSignature`, `PlatformSignature`, `DateTimeNumber`, `EncodedXyzNumber`, `ProfileId`, `S15Fixed16`, `CieXyz`, `ICC_MAGIC_NUMBER` |
| `context.rs` | `ErrorCode`, `CmsError`                                                                                                                                                                                             |

Pipeline/Curves モジュールへの依存はなし（Level 3 基盤）。

## 型定義

### IoHandler enum

```rust
/// I/O handler for reading/writing ICC profile data.
/// C版: `cmsIOHANDLER`
pub(crate) enum IoHandler {
    Null { pointer: u32, used_space: u32 },
    Memory {
        data: Vec<u8>,
        pointer: u32,
        used_space: u32,
        reported_size: u32,
        is_write: bool,
    },
    File {
        file: std::fs::File,
        used_space: u32,
        reported_size: u32,
        path: String,
    },
}
```

C版の `FILE*` ストリームハンドラ（`cmsOpenIOhandlerFromStream`）は移植しない（Rust に `FILE*` 相当がない）。

### TagEntry / TagData

```rust
const MAX_TABLE_TAG: usize = 100;

pub(crate) struct TagEntry {
    pub sig: TagSignature,
    pub offset: u32,
    pub size: u32,
    pub linked: Option<TagSignature>,
    pub data: TagDataState,
    pub save_as_raw: bool,
}

pub(crate) enum TagDataState {
    NotLoaded,
    Raw(Vec<u8>),
    Loaded(TagData),
}
```

### TagData enum（PR 4a での初期版）

```rust
pub enum TagData {
    Raw(Vec<u8>),
}
```

PR 4b でバリアント追加（Xyz, Curve, Mlu, Pipeline 等）。

### Profile

```rust
pub struct Profile {
    pub(crate) header: IccHeader,
    pub(crate) tags: Vec<TagEntry>,
    pub(crate) io: Option<IoHandler>,
    pub(crate) is_write: bool,
}
```

## C版→Rust 関数マッピング

### IoHandler（cmsio0.c lines 38-519）

| C版                                | Rust                                     |
| ---------------------------------- | ---------------------------------------- |
| `cmsOpenIOhandlerFromNULL`         | `IoHandler::new_null()`                  |
| `cmsOpenIOhandlerFromMem` (read)   | `IoHandler::from_memory_read(data)`      |
| `cmsOpenIOhandlerFromMem` (write)  | `IoHandler::from_memory_write(capacity)` |
| `cmsOpenIOhandlerFromFile` (read)  | `IoHandler::from_file_read(path)`        |
| `cmsOpenIOhandlerFromFile` (write) | `IoHandler::from_file_write(path)`       |
| `cmsCloseIOhandler`                | `Drop` impl                              |

### 数値 I/O ヘルパー（cmsplugin.c lines 111-431）

IoHandler のメソッドとして実装。すべてビッグエンディアン。

| C版                                        | Rust                                              |
| ------------------------------------------ | ------------------------------------------------- |
| `_cmsReadUInt8Number`                      | `io.read_u8()`                                    |
| `_cmsReadUInt16Number`                     | `io.read_u16()`                                   |
| `_cmsReadUInt32Number`                     | `io.read_u32()`                                   |
| `_cmsReadFloat32Number`                    | `io.read_f32()`                                   |
| `_cmsReadUInt64Number`                     | `io.read_u64()`                                   |
| `_cmsRead15Fixed16Number`                  | `io.read_s15fixed16()`                            |
| `_cmsReadXYZNumber`                        | `io.read_xyz()`                                   |
| `_cmsReadUInt16Array`                      | `io.read_u16_array(n)`                            |
| `_cmsWrite*` 系                            | 対応する `io.write_*()` メソッド                  |
| `_cmsReadAlignment` / `_cmsWriteAlignment` | `io.read_alignment()` / `io.write_alignment()`    |
| `_cmsReadTypeBase` / `_cmsWriteTypeBase`   | `io.read_type_base()` / `io.write_type_base(sig)` |

### Profile ライフサイクル（cmsio0.c lines 540-1645）

| C版                           | Rust                                |
| ----------------------------- | ----------------------------------- |
| `cmsCreateProfilePlaceholder` | `Profile::new_placeholder()`        |
| `_cmsReadHeader`              | `Profile::read_header()` — private  |
| `_cmsWriteHeader`             | `Profile::write_header()` — private |
| `cmsOpenProfileFromFileTHR`   | `Profile::open_file(path)`          |
| `cmsOpenProfileFromMemTHR`    | `Profile::open_mem(data)`           |
| `cmsSaveProfileToIOhandler`   | `Profile::save_to_io()` — private   |
| `cmsSaveProfileToFile`        | `Profile::save_to_file(path)`       |
| `cmsSaveProfileToMem`         | `Profile::save_to_mem()`            |
| `cmsCloseProfile`             | `Drop` impl                         |

### タグディレクトリ操作（cmsio0.c lines 595-2151）

| C版                  | Rust                                |
| -------------------- | ----------------------------------- |
| `cmsGetTagCount`     | `profile.tag_count()`               |
| `cmsGetTagSignature` | `profile.tag_signature(n)`          |
| `_cmsSearchTag`      | `profile.search_tag(sig)` — private |
| `_cmsNewTag`         | `profile.new_tag(sig)` — private    |
| `cmsIsTag`           | `profile.has_tag(sig)`              |
| `cmsReadRawTag`      | `profile.read_raw_tag(sig)`         |
| `cmsWriteRawTag`     | `profile.write_raw_tag(sig, data)`  |
| `cmsLinkTag`         | `profile.link_tag(sig, dest)`       |
| `cmsTagLinkedTo`     | `profile.tag_linked_to(sig)`        |

### ヘッダーアクセサ

C版のゲッター/セッター（`cmsGetPCS` 等）は、Rust では `profile.header` の直接フィールドアクセスで代替。追加で以下のメソッドを提供：

| C版                       | Rust                         |
| ------------------------- | ---------------------------- |
| `cmsGetProfileVersion`    | `profile.version_f64()`      |
| `cmsSetProfileVersion`    | `profile.set_version_f64(v)` |
| `cmsGetEncodedICCversion` | `profile.header.version`     |

### 保存ロジック（cmsio0.c lines 1334-1591）

| C版        | Rust                             |
| ---------- | -------------------------------- |
| `SaveTags` | `Profile::save_tags()` — private |
| `SetLinks` | `Profile::set_links()` — private |

2パスアルゴリズム：Pass 1 で Null ハンドラに書き込みオフセット計算、Pass 2 で実ハンドラに書き込み。

## コミット構成（TDD）

### Commit 1: RED — IoHandler テスト

```text
test(io): add IoHandler tests for Null, Memory, and File variants
```

- `null_handler_tracks_position`: write → tell → used_space 検証
- `null_handler_seek`: seek → tell 検証
- `memory_read_basic`: 既知バイト列の読み取り
- `memory_read_past_end`: バッファ末尾を超えた読み取り → false
- `memory_read_seek_and_tell`: seek + tell 検証
- `memory_write_basic`: 書き込み → バッファ内容検証
- `file_read_write_roundtrip`: tmp ファイル書き込み → 読み戻し

### Commit 2: GREEN — IoHandler 実装

```text
feat(io): implement IoHandler (Null, Memory, File variants)
```

### Commit 3: RED — 数値ヘルパーテスト

```text
test(io): add number read/write helper tests
```

- `read_write_u16_big_endian`: 0x1234 → bytes [0x12, 0x34] → 読み戻し
- `read_write_u32_big_endian`: 同パターン
- `read_write_f32_roundtrip`
- `read_write_s15fixed16_roundtrip`: D50 定数で検証
- `read_write_xyz_roundtrip`
- `read_write_u16_array_roundtrip`
- `write_alignment_pads_to_4bytes`
- `read_type_base_roundtrip`

### Commit 4: GREEN — 数値ヘルパー実装

```text
feat(io): implement number read/write helpers and alignment
```

### Commit 5: RED — Header / Tag Directory テスト

```text
test(io): add Profile header and tag directory tests
```

- `header_roundtrip_in_memory`: placeholder → ヘッダー設定 → save → reopen → 検証
- `header_magic_number_validation`: 不正マジック → open 失敗
- `tag_directory_roundtrip`: タグ追加 → save → reopen → tag_count / tag_signature 検証
- `tag_directory_max_100_tags`: 101個目 → 失敗
- `tag_linking`: link_tag → save → reopen → shared offset 検証

### Commit 6: GREEN — Header / Tag Directory 実装

```text
feat(io): implement Profile struct, header read/write, and tag directory
```

### Commit 7: RED — Profile ライフサイクルテスト

```text
test(io): add Profile open/save lifecycle tests
```

- `open_from_memory_test1_icc`: `reference/Little-CMS/testbed/test1.icc` を読み込み → ヘッダー検証
- `open_from_file_test1_icc`: ファイルパス経由で同上
- `placeholder_profile_defaults`: デフォルト値検証
- `save_to_memory_roundtrip`: raw タグ追加 → save → reopen → 検証
- `read_raw_tag_write_raw_tag`: raw バイト書き込み → 読み戻し
- `version_f64_encoding`: version 4.3 → encoded 0x04300000

### Commit 8: GREEN — Profile ライフサイクル実装

```text
feat(io): implement Profile open/save lifecycle and raw tag read/write
```

## エッジケース・エラー処理

- **不正マジックナンバー**: open 失敗、`ErrorCode::BadSignature`
- **バージョン > 5.0**: open 失敗、`ErrorCode::UnknownExtension`
- **タグ数 > 100**: ディレクトリ読み込み時にトランケート（C版動作に合わせる）
- **タグ offset+size がファイルサイズ超過**: 当該タグをスキップ
- **f32 の異常値**: |v| > 1E20 をリジェクト（C版動作）
- **アライメント**: 4バイト境界にゼロパディング
- **保存2パス**: Pass 1 で Null ハンドラでサイズ計算、Pass 2 で実書き込み
- **BCD バージョン検証**: 各ニブル 0-9 のみ許容

## 検証方法

```bash
cargo test io               # io モジュールテスト
cargo test                  # 全テスト（回帰確認）
cargo clippy --all-targets -- -D warnings
cargo fmt -- --check
```

テストプロファイル: `reference/Little-CMS/testbed/test1.icc`
