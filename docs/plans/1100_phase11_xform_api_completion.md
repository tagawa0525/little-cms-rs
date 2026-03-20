# Phase 11: Transform API Completion

**Status**: IMPLEMENTED
**C版ファイル**: `cmsxform.c`（残API）
**Rust見積**: ~200行（impl）+ ~100行（tests）
**ブランチ**: `feat/phase11-xform-api`

## Context

Phase 1-10 で ICC カラーマネジメントの中核機能（プロファイル I/O、パイプライン構築、変換実行、最適化、ガマットチェック）は完成している。
本フェーズでは xform.rs の残り公開 API を実装し、C版の `cmsxform.c` との機能パリティを達成する。

## スコープ

### 実装する機能

1. **`Transform::new_extended()`** — per-profile intent/BPC/adaptation + gamut check 付きの汎用コンストラクタ（C版: `cmsCreateExtendedTransform`）
2. **Null transform** — FLAGS_NULLTRANSFORM: パイプラインを介さず unpack→pack のみ実行
3. **1-pixel cache** — 直前の入出力をキャッシュし、同一入力の連続変換をスキップ（16bit のみ、float は NOCACHE 強制）
4. **`Transform::change_buffers_format()`** — 変換済みパイプラインのピクセルフォーマットを動的変更（C版: `cmsChangeBuffersFormat`）
5. **Linear RGB 16-bit optimization inhibit** — 16bit 線形 RGB 入力時に自動で NOOPTIMIZE を付与（γ < 1.6 検出）

### Deferred

- Stride-based transforms (`cmsDoTransformStride`, `cmsDoTransformLineStride`) — 内部ループの構造変更が必要なため別フェーズ
- Plugin transform functions — プラグインシステム未実装
- InputColorant / OutputColorant / Sequence 保持 — Named Color 統合時に実装

## 変更対象ファイル

| ファイル                 | 操作                                               |
| ------------------------ | -------------------------------------------------- |
| `src/transform/xform.rs` | new_extended, null transform, cache, change_format |

## 実装する関数

### xform.rs

| 関数                                 | C版                            | 内容                                       |
| ------------------------------------ | ------------------------------ | ------------------------------------------ |
| `Transform::new_extended()`          | `cmsCreateExtendedTransform`   | 汎用コンストラクタ（per-profile 配列）     |
| null transform パス                  | `NullXFORM` / `NullFloatXFORM` | unpack→pack のみ（パイプラインなし）       |
| 1-pixel cache                        | `CachedXFORM`                  | 16bit: memcmp → cache hit で eval スキップ |
| `Transform::change_buffers_format()` | `cmsChangeBuffersFormat`       | フォーマッタ動的差し替え                   |

## 処理フロー

### Transform::new_extended()

```text
1. nProfiles 範囲チェック (1..=255)
2. float フォーマット → FLAGS_NOCACHE 強制
3. FLAGS_NULLTRANSFORM → null transform 構築（パイプラインなし）
4. FLAGS_GAMUTCHECK → gamut profile + PCS position 検証
5. 16bit RGB 入力 + γ < 1.6 → FLAGS_NOOPTIMIZE 付与
6. link_profiles() でパイプライン構築（per-profile intents/BPC/adaptation）
7. チャネル数検証
8. 最適化
9. フォーマッタ選択
10. FLAGS_GAMUTCHECK → create_gamut_check_pipeline()
11. FLAGS_NOCACHE でなければキャッシュ初期化（input=0 で1回評価）
12. Transform 返却
```

### Null transform

```text
16bit パス:
  unpack(input) → w_in
  pack(w_in) → output  （パイプライン評価なし）

float パス:
  unpack(input) → w_in
  pack(w_in) → output  （パイプライン評価なし）
```

### 1-pixel cache

```text
16bit パス (cache 有効時):
  unpack → w_in
  w_in == cache_in → cache hit: w_out = cache_out
  w_in != cache_in → cache miss: pipeline.eval_16(w_in, w_out); cache 更新
  pack(w_out)

float パスでは cache 無効（FLAGS_NOCACHE 強制）
```

### change_buffers_format()

```text
1. 16bit フォーマッタのみ対応（float は不可）
2. 新しい input/output フォーマットのフォーマッタを検索
3. フォーマッタ差し替え + フォーマット値更新
4. チャネル数の一致は呼び出し元の責任
```

## コミット構成（TDD）

### Commit 1: 計画書

### Commit 2 (RED): テスト

- `null_transform_16bit`: FLAGS_NULLTRANSFORM で入力がそのまま出力
- `null_transform_float`: float 版の null transform
- `cache_hit_returns_same_output`: 同一入力で2回変換 → キャッシュヒット
- `cache_miss_updates_output`: 異なる入力で変換 → キャッシュミス → 正しい出力
- `new_extended_basic`: per-profile intent/BPC で基本変換
- `new_extended_gamut_check`: gamut profile 付き extended transform
- `change_buffers_format_basic`: RGB_8 → RGB_16 にフォーマット変更
- `change_buffers_format_rejects_float`: float フォーマットは拒否

### Commit 3 (GREEN): 実装

## 検証方法

```bash
cargo test xform
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt -- --check
```
