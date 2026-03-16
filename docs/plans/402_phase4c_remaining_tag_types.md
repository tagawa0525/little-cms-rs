# Phase 4c-1: 残りタグ型ハンドラ + ヘルパー

**Status**: PLANNED
**C版ファイル**: `cmstypes.c`（残り部分）
**Rust見積**: ~600行（impl）+ ~400行（tests）
**ブランチ**: `feat/phase4c-remaining-tags`

## Context

Phase 4b（コアタグ型20種）がマージ済み。残りの非 LUT タグ型とユーティリティを実装する。

Phase 4c の PR 分割:

- **PR 4c-1**（本計画）: 残りタグ型（UcrBg, CrdInfo, ProfileSequenceDesc/Id, vcgt, cicp, Dict）+ ヘルパー
- **PR 4c-2**（別計画）: LUT タグ型（Lut8, Lut16, LutAtoB, LutBtoA, MPE）
- **PR 4c-3**（別計画）: cmsio1.c パイプライン構築ヘルパー

## 変更対象ファイル

| ファイル | 操作 |
| --- | --- |
| `src/profile/tag_types.rs` | タグ型ハンドラ追加、dispatch 拡張 |
| `src/types.rs` | VideoSignalType 構造体追加 |

## 実装するタグ型

| TagTypeSignature | C版行数 | 内容 |
| --- | --- | --- |
| UcrBg | ~93行 | UCR/BG カーブ + 説明テキスト |
| CrdInfo | ~37行 | PostScript CRD 名（MLU） |
| ProfileSequenceDesc | ~74行 | プロファイルシーケンス記述 |
| ProfileSequenceId | ~50行 | プロファイルシーケンス ID |
| vcgt | ~184行 | ビデオカードガンマテーブル |
| VideoSignal (cicp) | ~36行 | ITU-R BT.2100 メタデータ |
| Dict | ~157行 | 名前-値ペア辞書 |

## ヘルパー

| ヘルパー | 用途 |
| --- | --- |
| `read_position_table` | オフセットテーブル読み込み（ProfileSequenceId, MPE で使用） |
| `write_position_table` | オフセットテーブル書き込み |
| `read_embedded_text` | バージョン依存テキスト読み込み（Text/TextDesc/MLU 自動判別） |

## コミット構成（TDD）

### Commit 1-2: UcrBg + CrdInfo

UcrBg: 2つの ToneCurve + 説明 MLU。CrdInfo: 固定キーの MLU。

### Commit 3-4: ProfileSequenceDesc/Id + position table + embedded text

position table ヘルパーと embedded text ヘルパーを含む。

### Commit 5-6: vcgt

ビデオカードガンマ（テーブル型/フォーミュラ型）。TagData に Vcgt バリアント追加。

### Commit 7-8: VideoSignal (cicp) + Dict

cicp は軽量。Dict は MLU と UTF-16 の組み合わせで中程度の複雑さ。

### Commit 9-10: dispatch 拡張 + 統合テスト

read_tag_type / write_tag_type の dispatch に新しい型を追加。

## 検証方法

```bash
cargo test tag_types
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt -- --check
```
