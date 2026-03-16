# Phase 4c-2: ProfileSequenceDesc/Id + vcgt + Dict タグ型

**Status**: PLANNED
**C版ファイル**: `cmstypes.c`（残り部分）
**Rust見積**: ~500行（impl）+ ~300行（tests）
**ブランチ**: `feat/phase4c2-seq-vcgt-dict`

## Context

Phase 4c-1（UcrBg, CrdInfo, VideoSignal）がマージ済み。4c-1 計画に含まれていた残りのタグ型を実装する。

## 変更対象ファイル

| ファイル | 操作 |
| --- | --- |
| `src/profile/tag_types.rs` | タグ型ハンドラ追加、dispatch 拡張 |
| `src/pipeline/named.rs` | ProfileSequenceDescEntry に profile_id/description フィールド追加 |

## 実装するタグ型

| TagTypeSignature | C版関数 | 内容 |
| --- | --- | --- |
| ProfileSequenceDesc | Type_ProfileSequenceDesc_Read/Write | プロファイルシーケンス記述（inline） |
| ProfileSequenceId | Type_ProfileSequenceId_Read/Write | プロファイルシーケンスID（position table） |
| vcgt | Type_vcgt_Read/Write | ビデオカードガンマテーブル（テーブル/フォーミュラ） |
| Dict | Type_Dictionary_Read/Write | 名前-値ペア辞書（UTF-16 + MLU） |

## ヘルパー

| ヘルパー | 用途 |
| --- | --- |
| `read_embedded_text` | バージョン依存テキスト読み込み（Text/TextDesc/MLU 自動判別） |
| `read_position_table` | オフセット/サイズテーブル読み込み（ProfileSequenceId で使用） |
| `write_position_table` | オフセット/サイズテーブル書き込み |

## データ構造変更

### ProfileSequenceDescEntry（named.rs）

```rust
pub struct ProfileSequenceDescEntry {
    pub device_mfg: u32,
    pub device_model: u32,
    pub attributes: u64,
    pub technology: Option<TechnologySignature>,
    pub manufacturer: Mlu,
    pub model: Mlu,
    pub profile_id: ProfileId,    // 追加: 16バイト MD5
    pub description: Mlu,         // 追加: ProfileSequenceId 用
}
```

### TagData enum（tag_types.rs）

新規バリアント:

- `ProfileSequenceDesc(ProfileSequenceDesc)`
- `Vcgt(Box<[ToneCurve; 3]>)` — 3チャンネル固定
- `Dict(Dict)`

## コミット構成（TDD）

### Commit 1-2: ProfileSequenceDesc + read_embedded_text

RED: roundtrip テスト（`#[ignore]`）
GREEN: read/write ハンドラ + read_embedded_text + dispatch

### Commit 3-4: ProfileSequenceId + position table

RED: roundtrip テスト
GREEN: position table ヘルパー + read/write ハンドラ + dispatch

### Commit 5-6: vcgt

RED: テーブル型・フォーミュラ型テスト
GREEN: read/write ハンドラ + dispatch

### Commit 7-8: Dict

RED: roundtrip テスト（DisplayName/DisplayValue あり/なし）
GREEN: read/write ハンドラ + dispatch

## 検証方法

```bash
cargo test tag_types
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt -- --check
```
