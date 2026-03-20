# Phase 12: DeltaE 拡張関数

**Status**: PLANNED
**C版ファイル**: `cmspcs.c`（DeltaE 関数群）
**Rust見積**: ~150行（impl）+ ~100行（tests）
**ブランチ**: `feat/phase12-delta-e`

## Context

Phase 1 で CIE76 DeltaE（`delta_e()`）を実装済み。
C版 `cmspcs.c` には追加で CIE94、BFD、CMC(l:c)、CIEDE2000 の4種のDeltaE関数があり、
色品質管理・プロファイル検証・色差評価で広く使われる。
純粋関数で依存なし、テストも明確なため独立フェーズとして実装する。

## スコープ

### 実装する関数

- `delta_e_cie94()` — C: `cmsCIE94DeltaE` — CIE94 色差
- `delta_e_bfd()` — C: `cmsBFDdeltaE` — BFD(1:1) 色差
- `delta_e_cmc()` — C: `cmsCMCdeltaE` — CMC(l:c) 色差
- `delta_e_ciede2000()` — C: `cmsCIE2000DeltaE` — CIEDE2000 色差

### 変更対象ファイル

| ファイル          | 操作           |
| ----------------- | -------------- |
| `src/math/pcs.rs` | 4関数 + helper |

## コミット構成（TDD）

### Commit 1: 計画書

### Commit 2 (RED): テスト

既知のLab値ペアに対するリファレンス値でテスト。

### Commit 3 (GREEN): 実装

## 検証方法

```bash
cargo test pcs
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt -- --check
```
