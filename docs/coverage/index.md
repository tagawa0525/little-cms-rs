# C版 Little CMS 移植カバレッジ

C版の各ソースファイルに対応するRust実装の関数レベルカバレッジ。

## カバレッジサマリー

| C ファイル                  | Rust ファイル                       | 公開API | 内部関数 | 実装率 | 状態                                  |
| --------------------------- | ----------------------------------- | ------- | -------- | ------ | ------------------------------------- |
| [cmserr.c](cmserr.md)       | `src/context.rs`                    | 3/5     | 1/8      | 部分的 | メモリ/Mutex関数はN/A                 |
| [cmshalf.c](cmshalf.md)     | `src/math/half.rs`                  | 2/2     | -        | 100%   | 完了                                  |
| [cmsmd5.c](cmsmd5.md)       | `src/math/md5.rs`                   | 4/4     | -        | 100%   | 完了                                  |
| [cmsmtrx.c](cmsmtrx.md)     | `src/math/mtrx.rs`                  | 12/12   | -        | 100%   | 完了                                  |
| [cmspcs.c](cmspcs.md)       | `src/math/pcs.rs`                   | 21/22   | 1/1      | 96%    | `cmsChannelsOf`のみ未実装（代替あり） |
| [cmsgamma.c](cmsgamma.md)   | `src/curves/gamma.rs`               | 23/23   | -        | 100%   | 完了                                  |
| [cmsintrp.c](cmsintrp.md)   | `src/curves/intrp.rs`               | 2/2     | 4/4      | 100%   | 完了                                  |
| [cmswtpnt.c](cmswtpnt.md)   | `src/curves/wtpnt.rs`               | 5/5     | 2/2      | 100%   | 完了                                  |
| [cmscam02.c](cmscam02.md)   | `src/curves/cam02.rs`               | 4/4     | -        | 100%   | 完了                                  |
| [cmslut.c](cmslut.md)       | `src/pipeline/lut.rs`               | 40/43   | 8/8      | 94%    | Context系未実装                       |
| [cmspack.c](cmspack.md)     | `src/pipeline/pack.rs`              | 2/3     | 2/4      | 部分的 | フォーマッタ基盤は実装済              |
| [cmsnamed.c](cmsnamed.md)   | `src/pipeline/named.rs`             | 26/29   | -        | 90%    | Wide文字列/NamedColor Stage未実装     |
| [cmsio0.c](cmsio0.md)       | `src/profile/io.rs`                 | 47/54   | 4/4      | 89%    | Stream系/一部公開APIなし              |
| [cmsio1.c](cmsio1.md)       | `src/profile/io.rs`                 | 8/9     | 5/5      | 93%    | wchar_t API のみ対象外                |
| [cmstypes.c](cmstypes.md)   | `src/profile/tag_types.rs`          | -       | 0/9      | 部分的 | 31タグ型実装/plugin系は設計差異       |
| [cmscnvrt.c](cmscnvrt.md)   | `src/transform/cnvrt.rs`            | 1/3     | 2/3      | 60%    | intent列挙/plugin未実装               |
| [cmsopt.c](cmsopt.md)       | `src/transform/opt.rs`              | 1/1     | 1/2      | 93%    | plugin登録のみ未実装                  |
| [cmssamp.c](cmssamp.md)     | `src/transform/samp.rs`             | 2/2     | -        | 100%   | 完了（dest BPは簡略版）               |
| [cmsgmt.c](cmsgmt.md)       | `src/transform/gmt.rs`              | 3/3     | 1/3      | 67%    | Chain2Lab/KToneCurve未実装            |
| [cmsxform.c](cmsxform.md)   | `src/transform/xform.rs`            | 19/29   | 1/8      | 54%    | worker系/global state未実装           |
| [cmsalpha.c](cmsalpha.md)   | `src/transform/alpha.rs`            | -       | 1/1      | 100%   | 完了                                  |
| [cmsvirt.c](cmsvirt.md)     | `src/profile/virt.rs`               | 22/22   | -        | 100%   | 完了                                  |
| [cmsplugin.c](cmsplugin.md) | `src/profile/io.rs`, `src/types.rs` | 28/37   | 2/4      | 73%    | plugin/context管理未実装              |
| [cmscgats.c](cmscgats.md)   | `src/ext/cgats.rs`                  | 24/38   | -        | 63%    | Multi/Cube未実装                      |
| [cmsps2.c](cmsps2.md)       | `src/ext/ps2.rs`                    | 2/3     | -        | 67%    | CSA/CRD実装済                         |
| [cmssm.c](cmssm.md)         | `src/transform/sm.rs`               | 5/5     | -        | 100%   | 完了                                  |

## 凡例

- **実装済**: Rust側に対応する実装が存在
- **N/A**: Rustの言語機能により不要（メモリ管理、Drop/Clone等）
- **設計差異**: C版のプラグインシステム等をRustでは異なるアプローチで実現
- **未実装**: 対応する実装が存在しない

## 全体的な設計差異

### Rustの言語機能で代替されるもの

- **メモリ管理関数** (`_cmsMalloc`, `_cmsFree` 等): Rustの所有権システムで代替
- **Free/Dup関数** (`cmsFreeToneCurve`, `cmsDupToneCurve` 等): `Drop`/`Clone` traitで代替
- **Mutex関数** (`_cmsCreateMutex` 等): Rustの`Mutex`型で代替
- **エンディアン変換** (`_cmsAdjustEndianess*`): `u16::from_be_bytes()` 等で代替

### 意図的に省略されているもの

- **プラグインシステム**: C版のハンドラ登録をRustでは `match` ディスパッチで代替
- **グローバルコンテキスト**: C版の `THR` なしバージョン（グローバルstate使用）は省略
- **Stream I/O**: `cmsOpenIOhandlerFromStream` 等は省略（ファイル/メモリI/Oでカバー）
- **Wide文字列**: `cmsMLUsetWide`/`cmsMLUgetWide` はUTF-8ベースのAPIで代替
