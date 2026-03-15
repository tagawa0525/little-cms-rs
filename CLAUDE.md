# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## little-cms-rs

C版 [Little CMS](https://github.com/mm2/Little-CMS) のRust移植。FFIバインディングではなく純粋な再実装。Rust edition 2024。
Little CMSはICC（International Color Consortium）仕様4.4のフル実装で、V2/V4プロファイルの読み書き、色空間変換、デバイスリンクプロファイル、名前付きカラーなどをサポートする軽量カラーマネジメントエンジン。

## ビルド・テスト・リント

```bash
cargo check
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt -- --check
cargo test tone_curve    # 特定テスト
```

## リファレンス

C版ソースは外部リポジトリを直接参照する（サブモジュールは使用しない）。`reference/` は `.gitignore` に含まれる。

- [mm2/Little-CMS](https://github.com/mm2/Little-CMS) — C版Little CMS（移植元）

### C版の主要ソース（`src/`）

| ファイル           | 内容                                                           |
| ------------------ | -------------------------------------------------------------- |
| `lcms2.h`          | 公開API定義（プロファイル・変換・パイプライン・色空間）        |
| `lcms2_plugin.h`   | プラグインAPI（14種の拡張ポイント）                            |
| `lcms2_internal.h` | 内部構造体・定数定義                                           |
| `cmsio0.c`         | プロファイルI/O基盤（ファイル・メモリ・ストリーム）            |
| `cmsio1.c`         | タグ読み書き・プロファイルヘッダ処理                           |
| `cmstypes.c`       | ICCタグ型のシリアライズ・デシリアライズ（最大ファイル）        |
| `cmspack.c`        | ピクセルフォーマッタ（8/16/32bit、planar/chunky変換）          |
| `cmscnvrt.c`       | レンダリングインテント実装・プロファイル間変換パイプライン構築 |
| `cmsintrp.c`       | 多次元LUT補間（trilinear、tetrahedral等）                      |
| `cmsopt.c`         | パイプライン最適化（カーブ結合、行列畳み込み等）               |
| `cmsxform.c`       | 色変換エンジン（パイプライン実行・並列化）                     |
| `cmsgamma.c`       | トーンカーブ（パラメトリック・テーブル・セグメント）           |
| `cmslut.c`         | パイプライン・ステージ管理（CLUT、行列、カーブセット）         |
| `cmsnamed.c`       | 名前付きカラーリスト・MLU（多言語Unicode）                     |
| `cmspcs.c`         | PCS色空間変換（XYZ↔Lab、エンコード/デコード）                  |
| `cmscam02.c`       | CIECAM02色覚モデル                                             |
| `cmsmtrx.c`        | 行列演算（3×3）                                                |
| `cmswtpnt.c`       | 白色点・色順応（Bradford変換）                                 |
| `cmsgmt.c`         | ガマットマッピング                                             |
| `cmssamp.c`        | CLUTサンプリング・スライシング                                 |
| `cmsvirt.c`        | 仮想プロファイル生成（sRGB、Lab、リンクプロファイル等）        |
| `cmsalpha.c`       | アルファチャネル処理                                           |
| `cmshalf.c`        | 半精度浮動小数点（IEEE 754-2008）                              |
| `cmsmd5.c`         | プロファイルID計算（MD5）                                      |
| `cmserr.c`         | エラーハンドリング・コンテキスト管理                           |
| `cmsplugin.c`      | プラグインレジストリ                                           |
| `cmsps2.c`         | PostScript CSA/CRD生成                                         |
| `cmscgats.c`       | CGATS（色測定データ）パーサ                                    |
| `cmssm.c`          | ガマットバウンダリ記述                                         |

### C版の外部依存

コアライブラリ（liblcms2）は外部依存ゼロ。libm・pthreadsのみ使用。Rust移植では標準ライブラリがこれらをカバーするため、コアは外部crateゼロで実装可能。

### ICC仕様書

ICC.1:2022 (Profile version 4.4) / ISO 15076-1

### ドキュメント

- `reference/Little-CMS/doc/LittleCMS2.18 API.pdf` — 公開API解説
- `reference/Little-CMS/doc/LittleCMS2.18 Plugin API.pdf` — プラグインAPI解説
- `reference/Little-CMS/doc/LittleCMS2.18 tutorial.pdf` — チュートリアル

## アーキテクチャ

### 色変換パイプライン

Little CMSの中核は、ICCプロファイル間の色変換をパイプラインとして構築・最適化・実行すること。

```text
■ 変換の流れ
入力ピクセル → Formatter(アンパック) → Pipeline[Stage→Stage→...] → Formatter(パック) → 出力ピクセル

■ パイプライン構成例（RGB→CMYK）
入力RGB → カーブセット(TRC) → 行列(RGB→XYZ) → XYZ→Lab → CLUT(Lab→CMYK) → カーブセット → 出力CMYK
```

### 主要概念

| 概念      | 説明                                                               |
| --------- | ------------------------------------------------------------------ |
| Profile   | ICCプロファイル。デバイスの色特性を記述（タグの集合）              |
| Transform | 2つ以上のプロファイル間の色変換。パイプラインを内包                |
| Pipeline  | ステージの連結リスト。色変換の実処理を担う                         |
| Stage     | パイプラインの1要素（カーブ、行列、CLUT等）                        |
| Formatter | ピクセルデータのパック/アンパック（8/16/float、planar等）          |
| Context   | スレッドセーフな状態管理。プラグイン・メモリ・エラーハンドラを保持 |
| Plugin    | 14種の拡張ポイント（補間、フォーマッタ、最適化等）                 |

### モジュール依存関係（移植順序）

```text
Level 0: err, half, md5, mtrx, pcs             （基盤ユーティリティ・色空間変換プリミティブ）
Level 1: gamma, intrp, wtpnt, cam02             （トーンカーブ・補間・色順応・色覚モデル）
Level 2: lut, pack, named                        （パイプライン/ステージ・フォーマッタ・名前付きカラー）
Level 3: io0, io1, types                         （プロファイルI/O・タグ型シリアライズ）
Level 4: cnvrt, opt, samp, gmt                   （変換構築・最適化・サンプリング・ガマット）
Level 5: xform, alpha                            （変換エンジン・アルファ処理）
Level 6: virt, plugin, cgats, ps2, sm            （仮想プロファイル・プラグイン・ユーティリティ）
```

下位レベルから順に移植する。上位モジュールは下位モジュールに依存するが逆はない。

### 主要データ構造

| C版                 | 概要                                                                           |
| ------------------- | ------------------------------------------------------------------------------ |
| `cmsHPROFILE`       | プロファイルハンドル（内部は `_cmsICCPROFILE`: ヘッダ・タグ辞書・IOハンドラ）  |
| `cmsHTRANSFORM`     | 変換ハンドル（内部は `_cmsTRANSFORM`: パイプライン・フォーマッタ・キャッシュ） |
| `cmsPipeline`       | ステージ連結リスト（16bit/float両評価パス）                                    |
| `cmsStage`          | 処理要素（カーブセット・行列・CLUT等、型によりデータが異なる）                 |
| `cmsToneCurve`      | トーンカーブ（セグメント・パラメトリック・16bitテーブル・補間パラメータ）      |
| `cmsContext`        | スレッドコンテキスト（16種のプラグインチャンク・メモリアロケータ）             |
| `cmsMLU`            | 多言語Unicodeテキスト（プロファイルメタデータ用）                              |
| `cmsNAMEDCOLORLIST` | 名前付きカラーパレット                                                         |
| `cmsICCHeader`      | ICCプロファイルヘッダ（128バイト固定長）                                       |

## PRワークフロー

### コミット構成

1. RED: テスト（`#[ignore = "not yet implemented"]` 付き）
2. GREEN: 実装（`#[ignore]` 除去）
3. REFACTOR: 必要に応じて
4. 全テスト・clippy・fmt通過を確認

### PR作成〜マージ

1. PR作成
2. `/gh-actions-check` でCopilotレビューワークフローが `completed/success` になるまで待つ
3. `/gh-pr-review` でコメント確認・対応
4. レビュー修正は独立した `fix(<scope>):` コミットで積む（RED/GREENに混入させない）
5. push後の再レビューサイクルも完了を確認
6. `docs/plans/` の進捗ステータスを更新（`docs:` コミット）
7. 全チェック通過後 `/gh-pr-merge --merge`

### 規約

- ブランチ命名: `feat/<module>-<機能>`, `test/<スコープ>`, `refactor/<スコープ>`, `docs/<スコープ>`
- コミット: Conventional Commits、scopeにモジュール名
- マージコミット: `## Why` / `## What` / `## Impact` セクション
- 計画書 (`docs/plans/`) を実装着手前にコミットすること

## 計画書

`docs/plans/NNN_<機能名>.md`（NNN = Phase番号×100 + 連番）。Status: PLANNED → IN_PROGRESS → IMPLEMENTED。C版の対応ファイル・関数を明記。
