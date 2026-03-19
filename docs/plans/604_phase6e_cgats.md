# Phase 6e: cgats.rs CGATS/IT8 パーサ

**Status**: PLANNED
**C版ファイル**: `cmscgats.c`（3,301行）
**Rust見積**: ~400行（impl）+ ~150行（tests）
**ブランチ**: `feat/phase6e-cgats`

## Context

CGATS (Committee for Graphic Arts Technologies Standards) / IT8 は
色測定データの標準テキストフォーマット。キャリブレーションターゲット、
測定データ、プロファイル検証に使用される。

## スコープ

C版の全機能ではなく、コアのパーサ+アクセサに絞る。

### 実装する機能

- IT8/CGATS テキストの解析（メモリから）
- ヘッダプロパティ（キー値ペア）の読み書き
- データテーブル（行×列）のアクセス
- テキストへのシリアライズ
- 複数テーブル対応

### Deferred

- CUBE フォーマット対応
- `$INCLUDE` ネスト読み込み
- 多値プロパティ（WRITE_PAIR）
- ファイル I/O（呼び出し側で `std::fs` を使用可能）
- 定義済みプロパティ/サンプルIDの検証

## 変更対象ファイル

| ファイル          | 操作     |
| ----------------- | -------- |
| `src/ext/mod.rs`  | 新規作成 |
| `src/ext/cgats.rs`| 新規作成 |
| `src/lib.rs`      | mod追加  |

## 実装する構造体・関数

### It8 構造体

- `It8::new()` — 空オブジェクト作成 (`cmsIT8Alloc`)
- `It8::load_from_str()` — テキスト解析 (`cmsIT8LoadFromMem`)
- `It8::save_to_string()` — テキスト書出し (`cmsIT8SaveToMem`)
- `table_count()` / `set_table()` — テーブル数・切替
- `sheet_type()` / `set_sheet_type()` — シートタイプ
- `set_property()` / `property()` / `property_f64()` — プロパティ読み書き
- `properties()` — プロパティ一覧
- `set_data_format()` — 列名定義
- `data_row_col()` / `set_data_row_col()` — セル値（行列指定）
- `data()` / `set_data()` — セル値（パッチ名指定）

## IT8 フォーマット概要

```text
CGATS.17                          ← シートタイプ
ORIGINATOR  "Instrument XYZ"      ← プロパティ
NUMBER_OF_FIELDS  4
NUMBER_OF_SETS    3
BEGIN_DATA_FORMAT
SAMPLE_ID  LAB_L  LAB_A  LAB_B   ← 列ヘッダ
END_DATA_FORMAT
BEGIN_DATA
A1  95.0  -0.5  1.2              ← データ行
A2  50.0  20.0  -10.0
A3  10.0  0.0   0.0
END_DATA
```

## コミット構成（TDD）

### Commit 1: 計画書

### Commit 2 (RED): テスト

- 基本的な IT8 テキスト解析
- プロパティの取得
- データテーブルアクセス（行列指定）
- パッチ名によるアクセス
- save → load round-trip
- 空テーブル

### Commit 3 (GREEN): 実装

## 検証方法

```bash
cargo test cgats
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt -- --check
```
