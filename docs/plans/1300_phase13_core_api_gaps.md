# Phase 13: Core API Gaps

Status: PLANNED

## Context

カバレッジレポート (`docs/coverage/`) により未実装関数が一覧化された。
プラグインシステムやグローバル状態等Rustで不要な関数を除外し、
実用上必要性の高い関数を優先度順に実装する。

## 対象

| # | 機能                   | C関数                                               | Rustファイル             | 規模  |
| - | ---------------------- | --------------------------------------------------- | ------------------------ | ----- |
| A | プロファイル情報取得   | `cmsGetProfileInfoASCII` / `cmsGetProfileInfoUTF8`  | `src/profile/io.rs`      | ~40行 |
| B | プロファイルID計算     | `cmsMD5computeID`                                   | `src/profile/io.rs`      | ~25行 |
| C | V2 Labエンコード       | `cmsLabEncoded2FloatV2` / `cmsFloat2LabEncodedV2`   | `src/math/pcs.rs`        | ~40行 |
| D | ストライド変換         | `cmsDoTransformLineStride` / `cmsDoTransformStride` | `src/transform/xform.rs` | ~80行 |
| E | CGATS ファイルI/O      | `cmsIT8LoadFromFile` / `cmsIT8SaveToFile`           | `src/ext/cgats.rs`       | ~20行 |
| F | プロファイルシーケンス | `_cmsRead/Write/CompileProfileSequence`             | `src/profile/io.rs`      | ~80行 |

## PR構成

### PR 1: feat/phase13a-core-api（A, B, C）

小さく独立した3つのコアAPI補完。依存関係なし。

#### A. `Profile::get_profile_info_ascii()` / `get_profile_info_utf8()`

`src/profile/io.rs` に `impl Profile` メソッド追加。

```rust
pub enum ProfileInfoType { Description, Manufacturer, Model, Copyright }
```

- info_type → TagSignature マッピング:
  - Description → `ProfileDescriptionML` (フォールバック: `ProfileDescription`)
  - Manufacturer → `DeviceMfgDesc`
  - Model → `DeviceModelDesc`
  - Copyright → `Copyright`
- `read_tag()` で MLU タグ読取 → `Mlu::get_ascii()` / `get_utf8()` で文字列取得
- C版参照: `cmsio1.c:985-1045`

#### B. `Profile::compute_md5_id()`

`src/profile/io.rs` に `impl Profile` メソッド追加。

1. `header.flags`, `header.rendering_intent`, `header.profile_id` を退避・ゼロ化
2. `save_to_mem()` でプロファイル全体をバイト列化
3. 元の値を復元
4. `Md5::digest(&blob)` でハッシュ計算
5. 結果を `header.profile_id` に格納

- C版参照: `cmsmd5.c:257-312`

#### C. `pcs_encoded_lab_to_float_v2()` / `float_to_pcs_encoded_lab_v2()`

`src/math/pcs.rs` に関数追加。

V2エンコード（V4との差異）:

- L*: 係数 652.8 (= 0xFF00/100.0)、V4は 655.35 (= 0xFFFF/100.0)
- a*/b*: V2もV4も係数 256.0 だが、V2のL*範囲は 0..0xFF00

```rust
pub fn float_to_pcs_encoded_lab_v2(lab: &CieLab) -> [u16; 3] {
    // L: clamp 0..100*(0xFFFF/0xFF00), then * 652.8
    // a,b: clamp -128..127.996, then (x+128)*256
}
pub fn pcs_encoded_lab_to_float_v2(encoded: &[u16; 3]) -> CieLab {
    // L: /652.8, a,b: /256-128
}
```

- C版参照: `cmspcs.c:178-265`

#### コミット順序

1. `test(pcs): add V2 Lab encoding tests` [RED]
2. `feat(pcs): implement V2 Lab encoding/decoding` [GREEN]
3. `test(profile): add get_profile_info tests` [RED]
4. `feat(profile): implement get_profile_info` [GREEN]
5. `test(profile): add compute_md5_id tests` [RED]
6. `feat(profile): implement compute_md5_id` [GREEN]

---

### PR 2: feat/phase13b-stride-transform（D）

ストライド対応変換。ホットパスの変更を含むため独立PR。

#### D. `Transform::do_transform_line_stride()`

`src/transform/xform.rs` にメソッド追加。

```rust
pub fn do_transform_line_stride(
    &self,
    input: &[u8],
    output: &mut [u8],
    pixels_per_line: usize,
    line_count: usize,
    bytes_per_line_in: usize,
    bytes_per_line_out: usize,
    bytes_per_plane_in: usize,
    bytes_per_plane_out: usize,
)
```

実装方針:

- `line_count` 回ループし、各行のオフセットを `line * bytes_per_line_{in,out}` で計算
- 各行で `pixels_per_line` 個のピクセルを既存の変換ロジックで処理
- フォーマッタの第4引数（現在常に`0`）に `bytes_per_plane_{in,out}` を渡す（planar形式用）
- `do_transform_stride()` は簡略版ラッパー
- alpha処理 (`handle_extra_channels`) もストライド対応が必要

既存の `do_transform_16` / `do_transform_float` を内部リファクタリングし、
stride情報を受け取れるようにする。`do_transform()` は `line_count=1` で呼び出す形に。

- C版参照: `cmsxform.c:211-249`

#### コミット順序

1. `test(xform): add stride-aware transform tests` [RED]
2. `refactor(xform): extract stride params in transform inner loop`
3. `feat(xform): implement do_transform_line_stride` [GREEN]

---

### PR 3: feat/phase13c-convenience（E, F）

便利関数。コアには影響しない。

#### E. `It8::load_from_file()` / `save_to_file()`

`src/ext/cgats.rs` にメソッド追加。`fs::read_to_string` / `fs::write` のラッパー。

#### F. `read_profile_sequence()` / `write_profile_sequence()` / `compile_profile_sequence()`

`src/profile/io.rs` に関数追加。

- `read_profile_sequence`: `ProfileSequenceDescTag` と `ProfileSequenceIdTag` を読み取り、

  両方存在すれば `profile_id` と `description` をマージして返す

- `write_profile_sequence`: `ProfileSequenceDescTag` に書き込み、v4以降は `ProfileSequenceIdTag` にも
- `compile_profile_sequence`: プロファイル配列からヘッダ情報・MLUタグを抽出してシーケンス構築

既存の型: `ProfileSequenceDesc` (`src/pipeline/named.rs`)、
タグI/O: `read_profile_sequence_desc_type` / `write_profile_sequence_desc_type` (`src/profile/tag_types.rs`)

- C版参照: `cmsio1.c:883-973`

#### コミット順序

1. `test(cgats): add file I/O tests` [RED]
2. `feat(cgats): implement load_from_file / save_to_file` [GREEN]
3. `test(profile): add profile sequence helper tests` [RED]
4. `feat(profile): implement read/write/compile profile sequence` [GREEN]

## 依存関係

```text
PR 1 (13a) ── 独立 ──→ 最初にマージ
PR 2 (13b) ── 独立 ──→ 2番目にマージ
PR 3 (13c) ── 弱依存 → 13a後にマージ推奨（Fの実装で get_profile_info を利用可能だが必須ではない）
```

## 検証

各PRで以下を確認:

```bash
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt -- --check
```

Phase 13a のテスト例:

- sRGBプロファイルのdescription/copyright取得 → 非空文字列
- MD5 ID計算の冪等性（同一プロファイル→同一ID）
- V2 Labエンコードのラウンドトリップ

Phase 13b のテスト例:

- パディング付きRGB8画像の行ストライド変換
- 連続バッファとストライド変換で同一結果

Phase 13c のテスト例:

- IT8ファイルの保存→読み込みラウンドトリップ
- 2つのsRGBプロファイルからシーケンスコンパイル
