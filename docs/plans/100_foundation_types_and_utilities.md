# Phase 1: 基盤型・ユーティリティ

- **Status**: IMPLEMENTED
- **Phase**: 1 (Level 0)
- **C版行数**: 2,680行
- **Rust概算**: ~2,500行
- **前提**: なし（最下位レベル）
- **ブランチ**: `feat/phase1-foundation`

## 概要

他の全モジュールが依存する基盤プリミティブ群を移植する。ICC署名enum・固定小数点型・色空間構造体・エラー処理・3×3行列・MD5・半精度浮動小数点・PCS色空間変換を含む。外部crate依存ゼロ。

## モジュール一覧と実装順序

Phase内の依存関係により、以下の順序で実装する。

```text
types ──→ context ──→ md5
  │                     ↑（_cmsMalloc等。ただしcmsMD5computeIDはPhase 4で実装）
  ├──→ mtrx
  ├──→ half           （完全独立）
  └──→ pcs            （types の CieXyz/CieLab/固定小数点を使用）
```

### 1. `src/types.rs` — ICC基本型・署名enum・固定小数点

**C版対応**: `lcms2.h` 型定義部分

#### 型定義

| C版                         | Rust版                                                         | 備考                                           |
| --------------------------- | -------------------------------------------------------------- | ---------------------------------------------- |
| `cmsS15Fixed16Number` (i32) | `S15Fixed16` newtype                                           | `Add`/`Sub`/`Mul`/`From<f64>`/`Into<f64>` 実装 |
| `cmsU16Fixed16Number` (u32) | `U16Fixed16` newtype                                           | 同上                                           |
| `cmsU8Fixed8Number` (u16)   | `U8Fixed8` newtype                                             | 同上                                           |
| `cmsCIEXYZ`                 | `CieXyz { x: f64, y: f64, z: f64 }`                            | フィールド名小文字                             |
| `cmsCIExyY`                 | `CieXyY { x: f64, y: f64, big_y: f64 }`                        | `Y`はRust慣用で`big_y`                         |
| `cmsCIELab`                 | `CieLab { l: f64, a: f64, b: f64 }`                            |                                                |
| `cmsCIELCh`                 | `CieLCh { l: f64, c: f64, h: f64 }`                            |                                                |
| `cmsJCh`                    | `JCh { j: f64, c: f64, h: f64 }`                               | CIECAM02用（Phase 2で使用）                    |
| `cmsCIEXYZTRIPLE`           | `CieXyzTriple { red, green, blue }`                            |                                                |
| `cmsCIExyYTRIPLE`           | `CieXyYTriple { red, green, blue }`                            |                                                |
| `cmsICCHeader`              | `IccHeader`                                                    | 128バイト固定長                                |
| `cmsProfileID`              | `ProfileId([u8; 16])`                                          | unionではなくバイト配列                        |
| `cmsDateTimeNumber`         | `DateTimeNumber { year, month, day, hours, minutes, seconds }` | 全`u16`                                        |
| `cmsEncodedXYZNumber`       | `EncodedXyzNumber { x, y, z }`                                 | 全`S15Fixed16`                                 |

#### ICC署名enum

全て `#[repr(u32)]` で定義。`TryFrom<u32>` を導出。

| C版                        | Rust版                  | バリアント数 |
| -------------------------- | ----------------------- | ------------ |
| `cmsTagTypeSignature`      | `TagTypeSignature`      | 36           |
| `cmsTagSignature`          | `TagSignature`          | ~70          |
| `cmsColorSpaceSignature`   | `ColorSpaceSignature`   | 37           |
| `cmsProfileClassSignature` | `ProfileClassSignature` | 10           |
| `cmsTechnologySignature`   | `TechnologySignature`   | 26           |
| `cmsPlatformSignature`     | `PlatformSignature`     | 6            |
| `cmsStageSignature`        | `StageSignature`        | 16           |
| `cmsCurveSegSignature`     | `CurveSegSignature`     | 3            |

#### ピクセルフォーマット

C版のビットフィールドマクロ群をRustの関数・定数として移植。

```rust
// PixelFormat は u32 newtype
pub struct PixelFormat(u32);

impl PixelFormat {
    pub const fn new() -> Self { ... }
    pub const fn colorspace(self) -> u32 { ... }
    pub const fn channels(self) -> u32 { ... }
    pub const fn bytes(self) -> u32 { ... }
    // ... 各フィールドのgetter/builder
}

// 定義済みフォーマット定数（~120個）
pub const TYPE_RGB_8: PixelFormat = ...;
pub const TYPE_CMYK_16: PixelFormat = ...;
```

#### 定数

```rust
pub const D50_X: f64 = 0.9642;
pub const D50_Y: f64 = 1.0;
pub const D50_Z: f64 = 0.8249;

pub const PERCEPTUAL_BLACK_X: f64 = 0.00336;
pub const PERCEPTUAL_BLACK_Y: f64 = 0.0034731;
pub const PERCEPTUAL_BLACK_Z: f64 = 0.00287;
```

#### レンダリングインテント

```rust
#[repr(u32)]
pub enum Intent {
    Perceptual = 0,
    RelativeColorimetric = 1,
    Saturation = 2,
    AbsoluteColorimetric = 3,
    // 非ICC拡張インテント（10-13, 200-203）
}
```

#### ピクセルタイプ定数

```rust
pub const PT_ANY: u32 = 0;
pub const PT_GRAY: u32 = 3;
pub const PT_RGB: u32 = 4;
// ... PT_CMY, PT_CMYK, PT_Lab 等
```

#### テスト（RED）

- 固定小数点round-trip: `S15Fixed16` — C版テスト値11個（正負・極値含む）、許容誤差 `1.0/65535.0`
- 固定小数点round-trip: `U8Fixed8` — C版テスト値6個、許容誤差 `1.0/255.0`
- 固定小数点round-trip: `U16Fixed16` — C版にテストなし、独自テスト値追加
- D50定数round-trip: `S15Fixed16`経由のD50値round-trip、許容誤差 `1e-5`
- `PixelFormat` のフィールド抽出・構築テスト
- ICC署名enumの `TryFrom<u32>` 往復テスト
- 基本型サイズ: `size_of::<S15Fixed16>() == 4` 等

---

### 2. `src/context.rs` — Context・エラーハンドリング

**C版対応**: `cmserr.c`（707行）

#### 設計判断

C版のContext構造体は16種のプラグインチャンクを `void*` 配列で保持する汎用設計だが、Rust版ではプラグインシステムが整備されるPhase 6まで簡素な構造にする。Phase 1時点では以下のみ:

- エラーハンドラコールバック
- エラーコードenum

C版の `_cmsMalloc` / `_cmsFree` / `_cmsSubAllocator` はRust標準アロケータで代替するため移植しない。C版の `_cmsCreateMutex` 等も `std::sync::Mutex` で代替。

#### エラーコード

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ErrorCode {
    Undefined = 0,
    File = 1,
    Range = 2,
    Internal = 3,
    Null = 4,
    Read = 5,
    Seek = 6,
    Write = 7,
    UnknownExtension = 8,
    ColorspaceCheck = 9,
    AlreadyDefined = 10,
    BadSignature = 11,
    CorruptionDetected = 12,
    NotSuitable = 13,
}
```

#### CmsError

```rust
#[derive(Debug)]
pub struct CmsError {
    pub code: ErrorCode,
    pub message: String,
}
```

#### Context

```rust
pub type LogErrorHandler = fn(error_code: ErrorCode, message: &str);

pub struct Context {
    error_handler: Option<LogErrorHandler>,
    alarm_codes: [u16; 16],          // ガマット警告色（Phase 5で使用）
    adaptation_state: f64,           // 絶対色域レンダリング用（Phase 5で使用）
}

impl Context {
    pub fn new() -> Self { ... }
    pub fn set_error_handler(&mut self, handler: LogErrorHandler) { ... }
    pub fn signal_error(&self, code: ErrorCode, message: &str) { ... }
}
```

グローバルデフォルトContextは後のPhaseで必要になった時点で `OnceLock<Mutex<Context>>` として追加。Phase 1では `Context` 構造体の定義と基本操作のみ。

#### 移植対象関数

| C版関数                    | Rust版                                     | 備考   |
| -------------------------- | ------------------------------------------ | ------ |
| `cmsSetLogErrorHandler`    | `Context::set_error_handler`               |        |
| `cmsSetLogErrorHandlerTHR` | 同上（Contextメソッド）                    |        |
| `cmsSignalError`           | `Context::signal_error`                    |        |
| `cmsGetEncodedCMMversion`  | `pub const VERSION: u32`                   | 定数化 |
| `_cmsTagSignature2String`  | `TagSignature::to_string` / `Display` impl |        |

#### 移植しない関数

| C版関数                                                                 | 理由                           |
| ----------------------------------------------------------------------- | ------------------------------ |
| `_cmsMalloc` / `_cmsFree` / `_cmsCalloc` / `_cmsRealloc` / `_cmsDupMem` | Rust標準アロケータ             |
| `_cmsCreateSubAlloc` / `_cmsSubAlloc` / `_cmsSubAllocDestroy`           | Rust `Vec`/`Box`               |
| `_cmsCreateMutex` / `_cmsLockMutex` 等                                  | `std::sync::Mutex`             |
| `_cmsCreateContext` / `_cmsDupContext` / `_cmsDeleteContext`            | Rustの所有権で管理             |
| `_cmsRegisterMemHandlerPlugin` / `_cmsRegisterMutexPlugin`              | Phase 6プラグインで検討        |
| `cmsfilelength`                                                         | `std::fs` / `Seek::stream_len` |
| `cmsstrcasecmp`                                                         | `str::eq_ignore_ascii_case`    |

#### テスト（RED）

- `Context::new()` でデフォルト状態確認
- `signal_error` でカスタムハンドラが呼ばれること
- `ErrorCode` の全バリアントが正しいu32値を持つこと

---

### 3. `src/math/mtrx.rs` — 3×3行列

**C版対応**: `cmsmtrx.c`（176行）

#### 型定義

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec3(pub [f64; 3]);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mat3(pub [Vec3; 3]);  // 行優先: Mat3.0[row].0[col]
```

#### 移植対象関数

| C版関数              | Rust版                                         | 備考                   |
| -------------------- | ---------------------------------------------- | ---------------------- |
| `_cmsVEC3init`       | `Vec3::new(x, y, z)`                           |                        |
| `_cmsVEC3minus`      | `impl Sub for Vec3`                            | `ops::Sub`             |
| `_cmsVEC3cross`      | `Vec3::cross(&self, other)`                    |                        |
| `_cmsVEC3dot`        | `Vec3::dot(&self, other)`                      |                        |
| `_cmsVEC3length`     | `Vec3::length(&self)`                          |                        |
| `_cmsVEC3distance`   | `Vec3::distance(&self, other)`                 |                        |
| `_cmsMAT3identity`   | `Mat3::identity()`                             |                        |
| `_cmsMAT3isIdentity` | `Mat3::is_identity(&self)`                     | 許容誤差 `1.0/65535.0` |
| `_cmsMAT3per`        | `impl Mul for Mat3`                            | `ops::Mul`             |
| `_cmsMAT3inverse`    | `Mat3::inverse(&self) -> Option<Mat3>`         | det < 0.0001 で `None` |
| `_cmsMAT3solve`      | `Mat3::solve(&self, b: &Vec3) -> Option<Vec3>` |                        |
| `_cmsMAT3eval`       | `Mat3::eval(&self, v: &Vec3) -> Vec3`          | 行列×ベクトル          |

#### テスト（RED）

C版にmatrix単体テストがないため、独自テストを作成:

- 単位行列の生成と `is_identity` 確認
- 行列×単位行列 = 元の行列
- 行列×逆行列 = 単位行列（既知の行列で検証）
- 特異行列の `inverse` が `None` を返すこと
- `Vec3` 演算: dot/cross積の既知値検証
- `solve`: 既知の連立方程式の解を検証
- `eval`: 既知の行列-ベクトル積を検証

---

### 4. `src/math/half.rs` — 半精度浮動小数点

**C版対応**: `cmshalf.c`（535行）

#### 設計判断

C版はルックアップテーブル方式（合計~12KB）。Rust版でも同方式を採用する。`f16`型はRust nightlyのみで安定版にはないため、`u16`表現のまま変換関数を提供。

#### 移植対象関数

| C版関数          | Rust版                         |
| ---------------- | ------------------------------ |
| `_cmsHalf2Float` | `half_to_float(h: u16) -> f32` |
| `_cmsFloat2Half` | `float_to_half(f: f32) -> u16` |

#### 実装方針

- 4つのルックアップテーブル（`Mantissa[2048]`, `Exponent[64]`, `Offset[64]`, `Base[512]`, `Shift[512]`）を `const` 配列として定義
- テーブルは `const fn` で生成するか、リテラル配列として記述する
- `f32`のビットパターン操作に `f32::to_bits()` / `f32::from_bits()` を使用（C版の `union` 相当）

#### テスト（RED）

- 全65536パターンのround-trip: `float_to_half(half_to_float(i)) == i`（NaN除外）
- 特殊値: `0`, `+Inf`, `-Inf`, 最小正の正規化数, 最大有限値
- `half_to_float(0) == 0.0`
- `half_to_float(0x3C00) == 1.0`（half-floatの1.0表現）

---

### 5. `src/math/md5.rs` — MD5ハッシュ

**C版対応**: `cmsmd5.c`（313行）

#### 設計判断

RFC 1321のMD5を直接実装する。外部crateは使わない。`cmsMD5computeID`（プロファイル全体のハッシュ）はプロファイルI/Oに依存するためPhase 4で実装。Phase 1ではMD5コア演算のみ。

#### 型定義

```rust
pub struct Md5 {
    buf: [u32; 4],     // A, B, C, D
    bits: [u32; 2],    // ビットカウント（low, high）
    input: [u8; 64],   // 入力バッファ
}
```

#### 移植対象関数

| C版関数           | Rust版                                | 備考                  |
| ----------------- | ------------------------------------- | --------------------- |
| `cmsMD5alloc`     | `Md5::new()`                          | メモリ確保不要        |
| `cmsMD5add`       | `Md5::update(&mut self, data: &[u8])` |                       |
| `cmsMD5finish`    | `Md5::finish(self) -> ProfileId`      | `self`を消費          |
| `cmsMD5computeID` | **Phase 4で実装**                     | プロファイルI/Oに依存 |

#### テスト（RED）

- RFC 1321 テストベクタ:
  - `MD5("") = d41d8cd98f00b204e9800998ecf8427e`
  - `MD5("a") = 0cc175b9c0f1b6a831c399e269772661`
  - `MD5("abc") = 900150983cd24fb0d6963f7d28e17f72`
  - `MD5("message digest") = f96b697d7cb7938d525a2f31aaf161d0`
  - `MD5("abcdefghijklmnopqrstuvwxyz") = c3fcd3d76192e4007dfb496cca67e13b`
- 分割投入: 同一データを1バイトずつ `update` した結果と一括投入の結果が一致すること

---

### 6. `src/math/pcs.rs` — PCS色空間変換

**C版対応**: `cmspcs.c`（949行）

#### 移植対象関数

**色空間変換**

| C版関数      | Rust版                                                  | 備考                                |
| ------------ | ------------------------------------------------------- | ----------------------------------- |
| `cmsXYZ2xyY` | `CieXyz::to_xy_y(&self) -> CieXyY`                      |                                     |
| `cmsxyY2XYZ` | `CieXyY::to_xyz(&self) -> CieXyz`                       |                                     |
| `cmsXYZ2Lab` | `CieXyz::to_lab(&self, white_point: &CieXyz) -> CieLab` | C版のNULL→D50デフォルトは引数で明示 |
| `cmsLab2XYZ` | `CieLab::to_xyz(&self, white_point: &CieXyz) -> CieXyz` |                                     |
| `cmsLab2LCh` | `CieLab::to_lch(&self) -> CieLCh`                       |                                     |
| `cmsLCh2Lab` | `CieLCh::to_lab(&self) -> CieLab`                       |                                     |

**Lab/XYZエンコード・デコード**

| C版関数                 | Rust版                                                 | 備考 |
| ----------------------- | ------------------------------------------------------ | ---- |
| `cmsLabEncoded2Float`   | `CieLab::from_encoded_v4(encoded: [u16; 3]) -> CieLab` |      |
| `cmsLabEncoded2FloatV2` | `CieLab::from_encoded_v2(encoded: [u16; 3]) -> CieLab` |      |
| `cmsFloat2LabEncoded`   | `CieLab::to_encoded_v4(&self) -> [u16; 3]`             |      |
| `cmsFloat2LabEncodedV2` | `CieLab::to_encoded_v2(&self) -> [u16; 3]`             |      |
| `cmsXYZEncoded2Float`   | `CieXyz::from_encoded(encoded: [u16; 3]) -> CieXyz`    |      |
| `cmsFloat2XYZEncoded`   | `CieXyz::to_encoded(&self) -> [u16; 3]`                |      |

**DeltaE**

| C版関数            | Rust版                                                                               | 備考  |
| ------------------ | ------------------------------------------------------------------------------------ | ----- |
| `cmsDeltaE`        | `CieLab::delta_e(&self, other: &CieLab) -> f64`                                      | CIE76 |
| `cmsCIE94DeltaE`   | `CieLab::delta_e_cie94(&self, other: &CieLab) -> f64`                                |       |
| `cmsBFDdeltaE`     | `CieLab::delta_e_bfd(&self, other: &CieLab) -> f64`                                  |       |
| `cmsCMCdeltaE`     | `CieLab::delta_e_cmc(&self, other: &CieLab, l: f64, c: f64) -> f64`                  |       |
| `cmsCIE2000DeltaE` | `CieLab::delta_e_ciede2000(&self, other: &CieLab, kl: f64, kc: f64, kh: f64) -> f64` |       |

**ユーティリティ**

| C版関数                                | Rust版                                                          | 備考               |
| -------------------------------------- | --------------------------------------------------------------- | ------------------ |
| `cmsD50_XYZ`                           | `CieXyz::d50() -> CieXyz`                                       |                    |
| `cmsD50_xyY`                           | `CieXyY::d50() -> CieXyY`                                       |                    |
| `cmsChannelsOfColorSpace`              | `ColorSpaceSignature::channels(&self) -> Option<u32>`           | enumメソッド       |
| `_cmsICCcolorSpace`                    | `ColorSpaceSignature::from_pixel_type(pt: u32) -> Option<Self>` |                    |
| `_cmsLCMScolorSpace`                   | `ColorSpaceSignature::to_pixel_type(&self) -> u32`              |                    |
| `_cmsReasonableGridpointsByColorspace` | Phase 3以降で実装                                               | パイプラインに依存 |
| `_cmsEndPointsBySpace`                 | Phase 3以降で実装                                               | パイプラインに依存 |

#### 内部ヘルパー

- `f(t: f64) -> f64` — Lab順方向変換関数。C版は `pow(t, 1.0/3.0)` を使用するが、Rust版は `f64::cbrt()` を使用（高速）
- `f_1(t: f64) -> f64` — Lab逆方向変換関数
- `atan2deg(a: f64, b: f64) -> f64` — atan2を度数で返す、[0, 360)正規化
- `quick_saturate_word(d: f64) -> u16` — `[0, 65535]` にクランプ＆ラウンド

#### エンコーディング定数

```rust
// V4 Lab encoding
const MAX_ENCODEABLE_L: f64 = 100.0;
const MIN_ENCODEABLE_AB_4: f64 = -128.0;
const MAX_ENCODEABLE_AB_4: f64 = 127.0;  // (65535.0/257.0) - 128.0 ≒ 127.0

// V2 Lab encoding
const MAX_ENCODEABLE_AB_2: f64 = 127.99609375;  // (65535.0/256.0) - 128.0

// XYZ encoding
const MAX_ENCODEABLE_XYZ: f64 = 1.99997;  // 1.0 + 32767.0/32768.0
```

#### テスト（RED）

**色空間変換（C版テスト移植）**

- Lab→XYZ→Lab round-trip: L∈[0,100]/10, a∈[-128,128]/8, b∈[-128,128]/8 の網羅的テスト。最大DeltaE < `1e-12`
- Lab→LCh→Lab round-trip: 同上の網羅的テスト。最大DeltaE < `1e-12`
- Lab→XYZ→xyY→XYZ→Lab round-trip: 同上。最大DeltaE < `1e-12`

**エンコーディング（C版テスト移植）**

- Lab V4 encoding round-trip: 全65535値、完全一致
- Lab V2 encoding round-trip: 全65535値、完全一致
- XYZ encoding round-trip: 独自テスト（C版になし）、代表的なXYZ値で検証

**DeltaE（C版にテストなし、独自作成）**

- CIE76: 同一色で0.0、既知のLab値ペアで期待値検証
- CIE94: 文献の参照値と比較
- CIEDE2000: Sharma et al. (2005) の34組の参照データセットと比較（最も信頼性の高い検証方法）
- CMC: 既知の参照値と比較
- BFD: 既知の参照値と比較

---

## ファイル構成

```text
src/
├── lib.rs           # pub mod types; pub mod context; pub mod math;
├── types.rs         # ICC基本型・署名enum・固定小数点・ピクセルフォーマット
├── context.rs       # Context・CmsError・ErrorCode
└── math/
    ├── mod.rs       # pub mod mtrx; pub mod half; pub mod md5; pub mod pcs;
    ├── mtrx.rs      # Vec3・Mat3
    ├── half.rs      # half_to_float・float_to_half
    ├── md5.rs       # Md5
    └── pcs.rs       # 色空間変換・DeltaE・エンコーディング
```

## コミット計画

TDDサイクルに従い、以下の順序でコミットする。

### types モジュール

| # | 種別  | コミットメッセージ                                       | 内容                                                                      |
| - | ----- | -------------------------------------------------------- | ------------------------------------------------------------------------- |
| 1 | RED   | `test(types): add fixed-point round-trip tests`          | S15Fixed16, U8Fixed8, U16Fixed16, D50 round-tripテスト（`#[ignore]`付き） |
| 2 | GREEN | `feat(types): implement fixed-point newtypes`            | 固定小数点型の実装、`#[ignore]`除去                                       |
| 3 | RED   | `test(types): add ICC signature and pixel format tests`  | 署名enum、PixelFormat テスト（`#[ignore]`付き）                           |
| 4 | GREEN | `feat(types): implement ICC signatures and pixel format` | 署名enum、PixelFormat、色空間構造体、ヘッダ型の実装、`#[ignore]`除去      |

### context モジュール

| # | 種別  | コミットメッセージ                                    | 内容                                                 |
| - | ----- | ----------------------------------------------------- | ---------------------------------------------------- |
| 5 | RED   | `test(context): add error handling tests`             | Context, ErrorCode テスト（`#[ignore]`付き）         |
| 6 | GREEN | `feat(context): implement context and error handling` | Context, CmsError, ErrorCode の実装、`#[ignore]`除去 |

### math モジュール群

| #  | 種別  | コミットメッセージ                                      | 内容                                                                      |
| -- | ----- | ------------------------------------------------------- | ------------------------------------------------------------------------- |
| 7  | RED   | `test(mtrx): add matrix and vector operation tests`     | Vec3, Mat3 テスト（`#[ignore]`付き）                                      |
| 8  | GREEN | `feat(mtrx): implement 3x3 matrix operations`           | Vec3, Mat3 の実装、`#[ignore]`除去                                        |
| 9  | RED   | `test(half): add half-float conversion tests`           | half_to_float, float_to_half テスト（`#[ignore]`付き）                    |
| 10 | GREEN | `feat(half): implement half-precision float conversion` | ルックアップテーブルと変換関数の実装、`#[ignore]`除去                     |
| 11 | RED   | `test(md5): add MD5 hash tests`                         | RFC 1321テストベクタ（`#[ignore]`付き）                                   |
| 12 | GREEN | `feat(md5): implement MD5 hash`                         | MD5の実装、`#[ignore]`除去                                                |
| 13 | RED   | `test(pcs): add color space conversion tests`           | XYZ↔Lab, Lab↔LCh, xyY, エンコーディング, DeltaE テスト（`#[ignore]`付き） |
| 14 | GREEN | `feat(pcs): implement PCS color space conversions`      | 全変換関数・DeltaEの実装、`#[ignore]`除去                                 |

## 完了基準

- `cargo test` — 全テスト通過
- `cargo clippy --all-targets -- -D warnings` — 警告ゼロ
- `cargo fmt -- --check` — フォーマット準拠
- 以下のテストが通過:
  - `CieXyz` / `CieLab` 相互変換（round-trip DeltaE < 1e-12）
  - 固定小数点演算（round-trip誤差が規定許容範囲内）
  - 全65535値のLabエンコーディングround-trip（完全一致）
  - DeltaE計算（参照値との一致）
  - MD5ハッシュ（RFC 1321テストベクタ一致）
  - half-float全65536パターンround-trip（NaN除外で完全一致）
  - 行列逆行列×元行列 ≈ 単位行列
