# Phase 6f: ps2.rs PostScript CSA/CRD 生成

**Status**: IMPLEMENTED
**C版ファイル**: `cmsps2.c`（1,603行）
**Rust見積**: ~500行（impl）+ ~100行（tests）
**ブランチ**: `feat/phase6f-ps2`

## Context

PostScript CSA (Color Space Array) と CRD (Color Rendering Dictionary) を
ICC プロファイルから生成する。PostScript RIP にプロファイル情報を埋め込む際に使用。

## スコープ

コアの CSA/CRD 生成に絞る。

### 実装する機能

- CSA 生成: matrix-shaper プロファイル（RGB → CIEBasedABC, Gray → CIEBasedA）
- CRD 生成: CLUT ベース（Lab → デバイス色空間）
- PostScript テンプレート出力（Lab↔XYZ 変換コード等）

### Deferred

- Named color プロファイルの CSA/CRD
- CIEBasedDEF / CIEBasedDEFG（CLUT ベース CSA）
- PQR ステージ（色順応）
- FLAGS_NODEFAULTRESOURCEDEF

## 変更対象ファイル

| ファイル         | 操作     |
| ---------------- | -------- |
| `src/ext/ps2.rs` | 新規作成 |
| `src/ext/mod.rs` | mod追加  |

## 実装する関数

- `get_postscript_csa()` — CSA 生成（`cmsGetPostScriptCSA`）
- `get_postscript_crd()` — CRD 生成（`cmsGetPostScriptCRD`）
- `emit_cie_based_a()` — Gray プロファイル用 PostScript
- `emit_cie_based_abc()` — RGB matrix-shaper 用 PostScript
- `emit_gamma()` — ガンマカーブの PostScript テーブル
- `emit_lab2xyz()` — Lab→XYZ 変換 PostScript コード
- `write_output_lut()` — CRD 用 CLUT 出力

## コミット構成（TDD）

### Commit 1: 計画書

### Commit 2 (RED): テスト

- sRGB プロファイル → CSA 生成、PostScript 構文含む
- Gray プロファイル → CSA 生成
- sRGB → Lab CRD 生成

### Commit 3 (GREEN): 実装

## 検証方法

```bash
cargo test ps2
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt -- --check
```
