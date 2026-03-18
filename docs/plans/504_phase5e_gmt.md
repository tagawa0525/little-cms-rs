# Phase 5e: gmt.rs ガマットマッピング

**Status**: IMPLEMENTED
**C版ファイル**: `cmsgmt.c`（662行）
**Rust見積**: ~350行（impl）+ ~150行（tests）
**ブランチ**: `feat/phase5e-gmt`

## Context

Phase 5d（opt.rs: パイプライン最適化）完了。
`cmsgmt.c` はガマットチェックパイプライン生成、K曲線構築（CMYK黒保持）、
TAC検出、RGBガンマ検出、Lab desaturation を実装する。

## 変更対象ファイル

| ファイル               | 操作                                   |
| ---------------------- | -------------------------------------- |
| `src/transform/gmt.rs` | 新規: ガマットマッピングユーティリティ |
| `src/transform/mod.rs` | `pub mod gmt;` 追加                    |

## 実装する関数

### gmt.rs — 公開API

| 関数                         | C版                        | 内容                                        |
| ---------------------------- | -------------------------- | ------------------------------------------- |
| `detect_tac()`               | `cmsDetectTAC`             | 出力プロファイルの Total Area Coverage 推定 |
| `detect_rgb_profile_gamma()` | `cmsDetectRGBProfileGamma` | RGBプロファイルのガンマ値推定               |
| `desaturate_lab()`           | `cmsDesaturateLab`         | Lab値のガマットプリズムクリッピング         |

### gmt.rs — 内部関数

| 関数                   | C版                   | 内容                       |
| ---------------------- | --------------------- | -------------------------- |
| `build_k_tone_curve()` | `_cmsBuildKToneCurve` | CMYK K曲線構築（黒保持用） |
| `compute_k_to_lstar()` | `ComputeKToLstar`     | K値→L*対応曲線計算         |
| `chain_to_lab()`       | `_cmsChain2Lab`       | プロファイル列→Lab変換作成 |

### Deferred

| 関数                            | C版                            | 理由                                                            |
| ------------------------------- | ------------------------------ | --------------------------------------------------------------- |
| `create_gamut_check_pipeline()` | `_cmsCreateGamutCheckPipeline` | FLAGS_GAMUTCHECK の Transform 統合が必要。xform.rs の拡張が前提 |
| `GamutSampler`                  | `GamutSampler`                 | 同上                                                            |

## 処理フロー

### detect_tac()

```text
1. Output クラス以外 → 0.0 返却
2. Lab→Profile 変換を作成（Perceptual intent）
3. Lab空間を6×74×74グリッドでサンプリング
4. 各グリッド点でプロファイル色素チャネルの合計を算出
5. 最大値をTAC（%）として返却
```

### detect_rgb_profile_gamma()

```text
1. Input/Display/Output/ColorSpace 以外 → -1.0
2. RGB以外 → -1.0
3. Profile→XYZ 変換を作成
4. 256点のグレーランプ（R=G=B）を変換
5. Y値からガンマを最小二乗フィッティング
6. 推定ガンマ値を返却（不明なら -1.0）
```

### desaturate_lab()

```text
1. L* < 0 → false
2. L* を [0, 100] にクリップ
3. a*, b* が [amin, amax], [bmin, bmax] 内 → true（変更なし）
4. 範囲外 → LCh変換して色相角ベースでクリップ
```

### build_k_tone_curve()

```text
1. 入力/出力が CMYK かつ Output クラスを検証
2. compute_k_to_lstar() で入力側 K→L* 曲線作成
3. compute_k_to_lstar() で出力側 K→L* 曲線作成
4. 2つのカーブを join して K_in → K_out 曲線生成
5. 単調性検証
```

## 既存モジュール依存

| 依存先               | 利用する関数                                                    |
| -------------------- | --------------------------------------------------------------- |
| `transform/xform.rs` | `Transform::new`, `Transform::new_multiprofile`, `do_transform` |
| `profile/io.rs`      | `Profile::new_lab4`, `save_to_mem`, `open_mem`                  |
| `profile/virt.rs`    | `Profile::new_lab4`                                             |
| `curves/gamma.rs`    | `ToneCurve::build_tabulated_float`, `join`, `is_monotonic`      |
| `math/pcs.rs`        | `lab_to_lch`, `xyz_to_lab`                                      |
| `pipeline/lut.rs`    | `slice_space_16`                                                |
| `transform/samp.rs`  | `formatter_for_colorspace`                                      |

## コミット構成（TDD）

### Commit 1: 計画書

### Commit 2 (RED): テスト

- `desaturate_lab`: 範囲内/外、L*クリップ、色相角ベースクリップ
- `detect_tac`: sRGBプロファイル → TAC ≈ 0 (非Outputクラス)
- `detect_rgb_profile_gamma`: sRGB → ガンマ ≈ 2.2

### Commit 3 (GREEN): 実装

## 検証方法

```bash
cargo test gmt
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt -- --check
```
