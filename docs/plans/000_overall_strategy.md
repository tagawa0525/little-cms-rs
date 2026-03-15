# Little CMS Rust移植 — 全体戦略計画書

## Context

Little CMS（liblcms2）はICC 4.4準拠の軽量カラーマネジメントエンジン。C版は26モジュール・約37,600行・公開API約300関数。外部依存ゼロ（libm/pthreads除く）という特性はRust移植に理想的で、標準ライブラリのみで実装可能。

本計画書はC版のモジュール依存関係を分析し、ボトムアップで移植するフェーズ分割と、Rustらしい設計判断の方針を定める。

## 設計方針

### C版との対応関係

- C版の1モジュール（`cmsXXX.c`）= Rustの1モジュール（`src/XXX.rs`）を基本とする
- 公開APIはRust慣用の命名・型に変換する（`cmsCreateTransform` → `Transform::new`等）
- C版のテストスイート（`testcms2.c`: 166テスト）を移植の正確性検証に使用する

### Rust設計判断

| 項目           | C版                               | Rust版                                              |
| -------------- | --------------------------------- | --------------------------------------------------- |
| エラー処理     | `cmsBool` + コールバック          | `Result<T, CmsError>`                               |
| 多態性         | 関数ポインタ構造体（vtable）      | trait object (`dyn Trait`)                          |
| メモリ管理     | malloc/free + プラグイン          | `Box`/`Arc`/`Vec`、カスタムアロケータはtrait        |
| スレッド安全性 | プラットフォーム別mutex           | `Arc<Mutex<T>>` / `RwLock<T>`                       |
| 固定小数点     | マクロ + bare `u32`/`i32`         | newtype（`S15Fixed16`, `U16Fixed16`）+ 算術ops実装  |
| 不透明ハンドル | `cmsHPROFILE`（void*）            | 所有型 `Profile` / `Transform`                      |
| I/O            | 関数ポインタ構造体                | `std::io::Read + Seek + Write` trait                |
| コンテキスト   | グローバル`Context0` + per-thread | `Context`構造体、デフォルトはグローバルシングルトン |
| ICC署名        | `#define` / `enum` (u32)          | `#[repr(u32)] enum`                                 |
| 初期化         | setter関数群                      | Builder pattern                                     |

### ライブラリ構成

機能レイヤーごとにディレクトリをグルーピングする。C版モジュールとの対応はコメントで示す。

```text
little-cms-rs/
├── src/
│   ├── lib.rs              # 公開API re-export
│   ├── types.rs            # ICC基本型・署名enum・固定小数点（lcms2.h型定義部分）
│   ├── context.rs          # Context・CmsError・メモリ管理（cmserr.c）
│   ├── math/               # 数学プリミティブ
│   │   ├── mod.rs
│   │   ├── mtrx.rs         # 3×3行列（cmsmtrx.c）
│   │   ├── md5.rs          # MD5ハッシュ（cmsmd5.c）
│   │   ├── half.rs         # f16変換（cmshalf.c）
│   │   └── pcs.rs          # XYZ↔Lab・DeltaE（cmspcs.c）
│   ├── curves/             # カーブ・補間
│   │   ├── mod.rs
│   │   ├── gamma.rs        # トーンカーブ（cmsgamma.c）
│   │   ├── intrp.rs        # LUT補間（cmsintrp.c）
│   │   ├── wtpnt.rs        # 白色点・色順応（cmswtpnt.c）
│   │   └── cam02.rs        # CIECAM02（cmscam02.c）
│   ├── pipeline/           # パイプライン・フォーマッタ
│   │   ├── mod.rs
│   │   ├── lut.rs          # Pipeline/Stage（cmslut.c）
│   │   ├── pack.rs         # ピクセルフォーマッタ（cmspack.c）
│   │   ├── named.rs        # MLU・名前付きカラー（cmsnamed.c）
│   │   └── samp.rs         # CLUTサンプリング（cmssamp.c）
│   ├── profile/            # プロファイルI/O
│   │   ├── mod.rs
│   │   ├── io.rs           # I/Oハンドラ・開閉（cmsio0.c + cmsio1.c）
│   │   ├── tag_types.rs    # タグ型ハンドラ（cmstypes.c）
│   │   └── virt.rs         # 仮想プロファイル生成（cmsvirt.c）
│   ├── transform/          # 変換エンジン
│   │   ├── mod.rs
│   │   ├── cnvrt.rs        # パイプライン構築（cmscnvrt.c）
│   │   ├── opt.rs          # 最適化（cmsopt.c）
│   │   ├── gmt.rs          # ガマットマッピング（cmsgmt.c）
│   │   ├── xform.rs        # 変換実行（cmsxform.c）
│   │   └── alpha.rs        # アルファチャネル（cmsalpha.c）
│   └── ext/                # 拡張機能（優先度低）
│       ├── mod.rs
│       ├── plugin.rs       # プラグインレジストリ（cmsplugin.c）
│       ├── cgats.rs        # CGATSパーサ（cmscgats.c）
│       ├── ps2.rs          # PostScript出力（cmsps2.c）
│       └── sm.rs           # ガマットバウンダリ（cmssm.c）
└── tests/
    ├── data/               # テスト用ICCプロファイル
    └── ...                 # モジュール別結合テスト
```

## フェーズ分割

依存関係の下位レベルから順に移植する。各フェーズ内のモジュールは独立して移植可能。

### Phase 1: 基盤型・ユーティリティ（Level 0）

外部依存ゼロのプリミティブ群。他の全モジュールの土台。

| モジュール | C版                  | 行数 | 内容                                                             |
| ---------- | -------------------- | ---- | ---------------------------------------------------------------- |
| `types`    | `lcms2.h` 型定義部分 | —    | ICC署名enum、固定小数点型、色空間型（`CieXyz`, `CieLab`等）      |
| `context`  | `cmserr.c`           | 707  | `Context`構造体、`CmsError` enum、エラーハンドラ、メモリ管理基盤 |
| `mtrx`     | `cmsmtrx.c`          | 176  | 3×3行列（乗算・逆行列・転置）                                    |
| `md5`      | `cmsmd5.c`           | 313  | プロファイルID計算                                               |
| `half`     | `cmshalf.c`          | 535  | f16↔f32変換テーブル                                              |
| `pcs`      | `cmspcs.c`           | 949  | XYZ↔Lab変換、DeltaE計算、エンコード/デコード                     |

**完了基準**: `CieXyz`/`CieLab`相互変換、固定小数点演算、DeltaE計算がテスト通過

### Phase 2: カーブ・補間・色順応（Level 1）

数学的処理の中核。Phase 1の型のみに依存。

| モジュール | C版          | 行数  | 内容                                                               |
| ---------- | ------------ | ----- | ------------------------------------------------------------------ |
| `gamma`    | `cmsgamma.c` | 1,514 | トーンカーブ（パラメトリック10種、テーブル、セグメント、逆カーブ） |
| `intrp`    | `cmsintrp.c` | 1,330 | 1D線形補間、3D trilinear/tetrahedral補間、4D-8D補間                |
| `wtpnt`    | `cmswtpnt.c` | 353   | Bradford/vonKries色順応行列、色温度↔白色点変換                     |
| `cam02`    | `cmscam02.c` | 490   | CIECAM02順方向・逆方向変換                                         |

**完了基準**: ガンマ2.2カーブ評価、3D LUT補間、D50↔D65色順応がテスト通過

### Phase 3: パイプライン・フォーマッタ（Level 2）

色変換エンジンの骨格。ステージ連結とピクセルデータの入出力。

| モジュール | C版          | 行数  | 内容                                                        |
| ---------- | ------------ | ----- | ----------------------------------------------------------- |
| `lut`      | `cmslut.c`   | 1,852 | Pipeline/Stage構造、カーブセット・行列・CLUTステージ        |
| `pack`     | `cmspack.c`  | 4,062 | ピクセルフォーマッタ（8/16/32bit、planar/chunky、アルファ） |
| `named`    | `cmsnamed.c` | 1,202 | MLU（多言語Unicode）、名前付きカラーリスト                  |

**完了基準**: パイプライン構築→評価（16bit/float）、RGB_8/CMYK_16等の基本フォーマット変換がテスト通過

### Phase 4: プロファイルI/O（Level 3）

ICCプロファイルの読み書き。ここで初めてファイルI/Oが入る。

| モジュール  | C版                     | 行数  | 内容                                                       |
| ----------- | ----------------------- | ----- | ---------------------------------------------------------- |
| `io`        | `cmsio0.c` + `cmsio1.c` | 3,196 | IOハンドラ抽象、プロファイル開閉、タグ辞書、ヘッダ読み書き |
| `tag_types` | `cmstypes.c`            | 6,252 | 全ICCタグ型のRead/Write/Dup/Freeハンドラ（最大モジュール） |

**完了基準**: sRGB.iccプロファイルの読み込み→タグ一覧取得→再書き出し→バイナリ一致がテスト通過

### Phase 5: 変換エンジン（Level 4-5）

プロファイル間の色変換を構築・最適化・実行する中核機能。

| モジュール | C版          | 行数  | 内容                                                         |
| ---------- | ------------ | ----- | ------------------------------------------------------------ |
| `cnvrt`    | `cmscnvrt.c` | 1,226 | レンダリングインテント実装、変換パイプライン構築             |
| `opt`      | `cmsopt.c`   | 1,992 | パイプライン最適化（カーブ結合、行列畳み込み、8bit高速パス） |
| `samp`     | `cmssamp.c`  | 599   | CLUTサンプリング・色空間スライシング                         |
| `gmt`      | `cmsgmt.c`   | 662   | ガマットマッピング・ガマットチェック                         |
| `xform`    | `cmsxform.c` | 1,475 | Transform構造体、`do_transform()`、並列実行                  |
| `alpha`    | `cmsalpha.c` | 650   | アルファチャネルコピー・変換                                 |

**完了基準**: `Transform::new(input_profile, output_profile, intent)` → `transform.apply(pixels)` でRGB→CMYK変換がテスト通過。C版と同一出力。

### Phase 6: 高レベル機能（Level 6）

コア機能の上に構築される便利機能群。個別に独立して移植可能。

| モジュール | C版           | 行数  | 優先度 | 内容                                         |
| ---------- | ------------- | ----- | ------ | -------------------------------------------- |
| `virt`     | `cmsvirt.c`   | 1,353 | 高     | sRGB/Lab/XYZプロファイル生成、デバイスリンク |
| `plugin`   | `cmsplugin.c` | 1,055 | 中     | プラグイン登録・ディスパッチ                 |
| `cgats`    | `cmscgats.c`  | 3,301 | 低     | CGATS/IT8テキストフォーマットパーサ          |
| `ps2`      | `cmsps2.c`    | 1,603 | 低     | PostScript CSA/CRD生成                       |
| `sm`       | `cmssm.c`     | 736   | 低     | ガマットバウンダリ記述                       |

**完了基準**: `Profile::new_srgb()` で生成したプロファイルを使った変換がテスト通過

## テスト戦略

### 各フェーズ共通

- C版テストスイート（`testcms2.c`）の該当テストをRustに移植
- テスト用ICCプロファイル（`test1.icc`〜`test5.icc`等）を `tests/data/` に配置
- 各モジュールはユニットテストを `#[cfg(test)] mod tests` に持つ

### 正確性検証

- C版の出力をゴールデンデータとして保存し、Rust版の出力と比較
- 浮動小数点比較は `1e-5` 程度の許容誤差を設定（C版との微小差異を許容）
- 固定小数点変換は完全一致を要求

### フェーズ完了判定

各フェーズで以下を全て通過:

```bash
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt -- --check
```

## 行数見積もり

| フェーズ | C版行数    | 概算（Rust） |
| -------- | ---------- | ------------ |
| Phase 1  | 2,680      | ~2,500       |
| Phase 2  | 3,687      | ~3,500       |
| Phase 3  | 7,116      | ~6,000       |
| Phase 4  | 9,448      | ~8,000       |
| Phase 5  | 6,604      | ~5,500       |
| Phase 6  | 8,048      | ~6,500       |
| **合計** | **37,583** | **~32,000**  |

Rustは型推論・パターンマッチ・イテレータにより定型コードが減る一方、エラーハンドリングの明示化で増える部分もある。全体としてC版の85%程度を見込む。

## 移植の進め方

1. フェーズごとに計画書（`docs/plans/NNN_<機能名>.md`）を作成してからコミット
2. 各モジュールはTDDサイクル（RED→GREEN→REFACTOR）でコミット
3. 1つのPRに1フェーズ（大きい場合はサブフェーズに分割）
4. Phase 1-5が完了すれば、ICCプロファイル間の色変換という基本機能が動作する
5. Phase 6は必要に応じて優先度順に実装
