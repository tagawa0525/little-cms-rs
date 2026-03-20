// ============================================================================
// PostScript CSA/CRD generation (C版: cmsps2.c)
// ============================================================================
//
// Generates PostScript Color Space Arrays (CSA) and Color Rendering
// Dictionaries (CRD) from ICC profiles.

use crate::context::CmsError;
use crate::curves::gamma::ToneCurve;
use crate::pipeline::lut::Pipeline;
use crate::profile::io::Profile;
use crate::profile::tag_types::TagData;
use crate::types::*;
use std::fmt::Write;

/// PostScript resource type selector.
/// C版: `cmsPSResourceType`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PostScriptResourceType {
    Csa,
    Crd,
}

// ============================================================================
// Public API
// ============================================================================

/// Generate a PostScript Color Space Array (CSA) from a profile.
/// CSA describes how to convert from device color to CIE XYZ/Lab.
/// C版: `cmsGetPostScriptCSA`
pub fn get_postscript_csa(
    profile: &mut Profile,
    intent: u32,
    _flags: u32,
) -> Result<String, CmsError> {
    let cs = profile.header.color_space;

    // Try matrix-shaper path first (smaller, more precise)
    if profile.is_matrix_shaper() {
        return match cs {
            ColorSpaceSignature::GrayData => generate_csa_gray(profile),
            ColorSpaceSignature::RgbData => generate_csa_rgb(profile),
            _ => generate_csa_lut(profile, intent),
        };
    }

    generate_csa_lut(profile, intent)
}

/// Dispatch to CSA or CRD generation based on resource type.
/// C版: `cmsGetPostScriptColorResource`
pub fn get_postscript_color_resource(
    _resource_type: PostScriptResourceType,
    _profile: &mut Profile,
    _intent: u32,
    _flags: u32,
) -> Result<Vec<u8>, CmsError> {
    todo!("Phase 14a-B: not yet implemented")
}

/// Generate a PostScript Color Rendering Dictionary (CRD) from a profile.
/// CRD describes how to convert from CIE XYZ/Lab to device color.
/// C版: `cmsGetPostScriptCRD`
pub fn get_postscript_crd(
    profile: &mut Profile,
    intent: u32,
    _flags: u32,
) -> Result<String, CmsError> {
    generate_crd(profile, intent)
}

// ============================================================================
// CSA generators
// ============================================================================

/// CSA for Gray matrix-shaper profile → /CIEBasedA
fn generate_csa_gray(profile: &mut Profile) -> Result<String, CmsError> {
    let mut out = String::new();

    // Read gray TRC
    let trc = match profile.read_tag(TagSignature::GrayTRC)? {
        TagData::Curve(c) => c,
        _ => {
            return Err(CmsError {
                code: crate::context::ErrorCode::NotSuitable,
                message: "GrayTRC tag not a curve".into(),
            });
        }
    };

    // Read white point
    let wp = read_white_point(profile);

    writeln!(out, "/CIEBasedA").unwrap();
    writeln!(out, "<<").unwrap();

    // Decode ABC (gamma table for gray)
    write!(out, "/DecodeA ").unwrap();
    emit_gamma(&mut out, &trc);
    writeln!(out).unwrap();

    // Matrix A maps gray to XYZ (Y component only → equal energy white)
    writeln!(out, "/MatrixA [{} {} {}]", wp.x, wp.y, wp.z).unwrap();

    // Range A
    writeln!(out, "/RangeA [0 1]").unwrap();

    // White point
    writeln!(out, "/WhitePoint [{} {} {}]", wp.x, wp.y, wp.z).unwrap();

    writeln!(out, ">>").unwrap();

    Ok(out)
}

/// CSA for RGB matrix-shaper profile → /CIEBasedABC
fn generate_csa_rgb(profile: &mut Profile) -> Result<String, CmsError> {
    let mut out = String::new();

    // Read TRCs
    let r_trc = read_curve_tag(profile, TagSignature::RedTRC)?;
    let g_trc = read_curve_tag(profile, TagSignature::GreenTRC)?;
    let b_trc = read_curve_tag(profile, TagSignature::BlueTRC)?;

    // Read colorant matrix columns (XYZ values)
    let r_col = read_xyz_tag(profile, TagSignature::RedMatrixColumn)?;
    let g_col = read_xyz_tag(profile, TagSignature::GreenMatrixColumn)?;
    let b_col = read_xyz_tag(profile, TagSignature::BlueMatrixColumn)?;

    // White point = sum of columns
    let wp = CieXyz {
        x: r_col.x + g_col.x + b_col.x,
        y: r_col.y + g_col.y + b_col.y,
        z: r_col.z + g_col.z + b_col.z,
    };

    writeln!(out, "/CIEBasedABC").unwrap();
    writeln!(out, "<<").unwrap();

    // DecodeABC (3 gamma tables)
    writeln!(out, "/DecodeABC [").unwrap();
    emit_gamma(&mut out, &r_trc);
    writeln!(out).unwrap();
    emit_gamma(&mut out, &g_trc);
    writeln!(out).unwrap();
    emit_gamma(&mut out, &b_trc);
    writeln!(out).unwrap();
    writeln!(out, "]").unwrap();

    // MatrixABC (RGB→XYZ, column-major for PostScript)
    writeln!(out, "/MatrixABC [").unwrap();
    writeln!(out, "{} {} {}", r_col.x, r_col.y, r_col.z).unwrap();
    writeln!(out, "{} {} {}", g_col.x, g_col.y, g_col.z).unwrap();
    writeln!(out, "{} {} {}", b_col.x, b_col.y, b_col.z).unwrap();
    writeln!(out, "]").unwrap();

    // RangeABC
    writeln!(out, "/RangeABC [0 1 0 1 0 1]").unwrap();

    // White/Black points
    writeln!(out, "/WhitePoint [{} {} {}]", wp.x, wp.y, wp.z).unwrap();

    writeln!(out, ">>").unwrap();

    Ok(out)
}

/// CSA for CLUT-based profiles (fallback)
fn generate_csa_lut(profile: &mut Profile, intent: u32) -> Result<String, CmsError> {
    let mut out = String::new();

    let pipeline = profile.read_input_lut(intent)?;
    let n_in = pipeline.input_channels() as usize;

    // Use CIEBasedDEF (3-input) or CIEBasedDEFG (4-input)
    let cs_name = match n_in {
        3 => "/CIEBasedDEF",
        4 => "/CIEBasedDEFG",
        _ => "/CIEBasedABC",
    };

    writeln!(out, "{cs_name}").unwrap();
    writeln!(out, "<<").unwrap();

    // Emit pipeline as a lookup table
    writeln!(out, "% CLUT-based color space (intent {})", intent).unwrap();

    let wp = read_white_point(profile);
    writeln!(out, "/WhitePoint [{} {} {}]", wp.x, wp.y, wp.z).unwrap();

    // Write range
    let range_str: String = (0..n_in).map(|_| "0 1 ").collect();
    writeln!(out, "/RangeABC [{}]", range_str.trim()).unwrap();

    // Emit the lookup table
    write_pipeline_as_table(&mut out, &pipeline, n_in);

    writeln!(out, ">>").unwrap();

    Ok(out)
}

// ============================================================================
// CRD generator
// ============================================================================

/// Generate CRD from profile. Always CLUT-based using Lab input.
fn generate_crd(profile: &mut Profile, intent: u32) -> Result<String, CmsError> {
    let mut out = String::new();

    let pipeline = profile.read_output_lut(intent)?;
    let n_out = pipeline.output_channels() as usize;

    writeln!(out, "<<").unwrap();
    writeln!(out, "/ColorRenderingType 1").unwrap();

    // White point (D50)
    writeln!(out, "/WhitePoint [{} {} {}]", D50_X, D50_Y, D50_Z).unwrap();
    writeln!(out, "/BlackPoint [0 0 0]").unwrap();

    // Encoding ABC (Lab → 0..1 normalization)
    writeln!(out, "/EncodeABC [").unwrap();
    writeln!(out, "  {{ 100 div }} bind  % L* / 100").unwrap();
    writeln!(out, "  {{ 128 add 256 div }} bind  % (a* + 128) / 256").unwrap();
    writeln!(out, "  {{ 128 add 256 div }} bind  % (b* + 128) / 256").unwrap();
    writeln!(out, "]").unwrap();

    // RenderTable
    write_render_table(&mut out, &pipeline, n_out);

    writeln!(out, ">>").unwrap();

    Ok(out)
}

// ============================================================================
// PostScript emitters
// ============================================================================

/// Emit a gamma curve as a PostScript procedure or lookup table.
fn emit_gamma(out: &mut String, curve: &ToneCurve) {
    let gamma = curve.estimate_gamma(0.1);

    if (gamma - 1.0).abs() < 0.01 {
        // Linear: identity function
        write!(out, "{{ }}").unwrap();
        return;
    }

    if gamma > 0.0 && (gamma - gamma.round()).abs() < 0.01 {
        // Simple power: use PostScript exp operator
        write!(out, "{{ {:.4} exp }}", gamma).unwrap();
        return;
    }

    // General case: emit lookup table (256 entries)
    write!(out, "{{ ").unwrap();
    write!(out, "<").unwrap();
    for i in 0..256 {
        let x = i as f64 / 255.0;
        let y = curve.eval_f32(x as f32) as f64;
        let v = (y * 255.0 + 0.5).clamp(0.0, 255.0) as u8;
        write!(out, "{v:02x}").unwrap();
        if (i + 1) % 32 == 0 && i < 255 {
            writeln!(out).unwrap();
        }
    }
    write!(out, ">").unwrap();
    write!(out, " dup length 1 sub 3 -1 roll mul round cvi get 255 div").unwrap();
    write!(out, " }}").unwrap();
}

/// Write a pipeline as a PostScript lookup table comment.
/// For CSA CLUT-based paths.
fn write_pipeline_as_table(out: &mut String, pipeline: &Pipeline, n_in: usize) {
    let n_out = pipeline.output_channels() as usize;

    // Sample the pipeline at grid points and emit as hex data
    let grid = 17u32; // Standard grid resolution
    let total = grid.pow(n_in as u32) as usize;

    writeln!(out, "/Table [").unwrap();

    let mut input = vec![0.0f32; n_in.max(MAX_CHANNELS)];
    let mut output = vec![0.0f32; n_out.max(MAX_CHANNELS)];

    for i in 0..total {
        // Compute grid coordinates
        let mut idx = i;
        for ch in (0..n_in).rev() {
            input[ch] = (idx % grid as usize) as f32 / (grid as f32 - 1.0);
            idx /= grid as usize;
        }

        pipeline.eval_float(&input, &mut output);

        write!(out, "<").unwrap();
        for val in &output[..n_out] {
            let v = (*val * 255.0 + 0.5).clamp(0.0, 255.0) as u8;
            write!(out, "{v:02x}").unwrap();
        }
        write!(out, "> ").unwrap();

        if (i + 1) % 8 == 0 {
            writeln!(out).unwrap();
        }
    }

    writeln!(out, "]").unwrap();
}

/// Write CRD render table from pipeline.
fn write_render_table(out: &mut String, pipeline: &Pipeline, n_out: usize) {
    let n_in = pipeline.input_channels() as usize;
    let grid = 17u32;

    writeln!(out, "/RenderTable [").unwrap();
    writeln!(out, "  {grid} % grid points").unwrap();

    let mut input = vec![0.0f32; n_in.max(MAX_CHANNELS)];
    let mut output = vec![0.0f32; n_out.max(MAX_CHANNELS)];

    let total = grid.pow(n_in as u32) as usize;

    for i in 0..total {
        let mut idx = i;
        for ch in (0..n_in).rev() {
            input[ch] = (idx % grid as usize) as f32 / (grid as f32 - 1.0);
            idx /= grid as usize;
        }

        pipeline.eval_float(&input, &mut output);

        write!(out, "<").unwrap();
        for val in &output[..n_out] {
            let v = (*val * 255.0 + 0.5).clamp(0.0, 255.0) as u8;
            write!(out, "{v:02x}").unwrap();
        }
        write!(out, "> ").unwrap();

        if (i + 1) % 8 == 0 {
            writeln!(out).unwrap();
        }
    }

    // Output decode functions (identity)
    writeln!(out, "  {} % output channels", n_out).unwrap();
    for _ in 0..n_out {
        writeln!(out, "  {{ }} bind").unwrap();
    }
    writeln!(out, "]").unwrap();
}

// ============================================================================
// Helpers
// ============================================================================

fn read_white_point(profile: &mut Profile) -> CieXyz {
    if let Ok(TagData::Xyz(wp)) = profile.read_tag(TagSignature::MediaWhitePoint) {
        wp
    } else {
        CieXyz {
            x: D50_X,
            y: D50_Y,
            z: D50_Z,
        }
    }
}

fn read_curve_tag(profile: &mut Profile, sig: TagSignature) -> Result<ToneCurve, CmsError> {
    match profile.read_tag(sig)? {
        TagData::Curve(c) => Ok(c),
        _ => Err(CmsError {
            code: crate::context::ErrorCode::NotSuitable,
            message: format!("{sig:?} is not a curve"),
        }),
    }
}

fn read_xyz_tag(profile: &mut Profile, sig: TagSignature) -> Result<CieXyz, CmsError> {
    match profile.read_tag(sig)? {
        TagData::Xyz(xyz) => Ok(xyz),
        _ => Err(CmsError {
            code: crate::context::ErrorCode::NotSuitable,
            message: format!("{sig:?} is not XYZ"),
        }),
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip(p: &mut Profile) -> Profile {
        let data = p.save_to_mem().unwrap();
        Profile::open_mem(&data).unwrap()
    }

    #[test]
    fn csa_from_srgb() {
        let mut p = roundtrip(&mut Profile::new_srgb());
        let csa = get_postscript_csa(&mut p, 0, 0).unwrap();
        assert!(csa.contains("/CIEBasedABC"), "should be CIEBasedABC");
        assert!(csa.contains("/DecodeABC"), "should have decode");
        assert!(csa.contains("/MatrixABC"), "should have matrix");
        assert!(csa.contains("/WhitePoint"), "should have white point");
    }

    #[test]
    fn csa_from_gray() {
        let gamma = ToneCurve::build_gamma(2.2).unwrap();
        let mut p = roundtrip(&mut Profile::new_gray(&crate::profile::virt::D65, &gamma));
        let csa = get_postscript_csa(&mut p, 0, 0).unwrap();
        assert!(csa.contains("/CIEBasedA"), "should be CIEBasedA");
        assert!(csa.contains("/DecodeA"), "should have decode");
    }

    #[test]
    fn csa_contains_gamma_info() {
        let mut p = roundtrip(&mut Profile::new_srgb());
        let csa = get_postscript_csa(&mut p, 0, 0).unwrap();
        // sRGB has parametric gamma, should emit lookup table
        assert!(
            csa.contains("exp") || csa.contains('<'),
            "should contain gamma (exp or lookup table)"
        );
    }

    #[test]
    fn crd_from_srgb() {
        let mut p = roundtrip(&mut Profile::new_srgb());
        let crd = get_postscript_crd(&mut p, 0, 0).unwrap();
        assert!(crd.contains("/ColorRenderingType"), "should have CRD type");
        assert!(crd.contains("/WhitePoint"), "should have white point");
        assert!(crd.contains("/EncodeABC"), "should have Lab encoding");
        assert!(crd.contains("/RenderTable"), "should have render table");
    }

    #[test]
    fn csa_output_is_nonempty() {
        let mut p = roundtrip(&mut Profile::new_srgb());
        let csa = get_postscript_csa(&mut p, 0, 0).unwrap();
        assert!(csa.len() > 50, "CSA should have meaningful content");
    }

    #[test]
    fn crd_output_is_nonempty() {
        let mut p = roundtrip(&mut Profile::new_srgb());
        let crd = get_postscript_crd(&mut p, 0, 0).unwrap();
        assert!(crd.len() > 50, "CRD should have meaningful content");
    }

    // ========================================================================
    // get_postscript_color_resource (Phase 14a-B)
    // ========================================================================

    #[test]
    #[ignore = "not yet implemented"]
    fn color_resource_csa_dispatches() {
        let mut p = roundtrip(&mut Profile::new_srgb());
        let result =
            get_postscript_color_resource(PostScriptResourceType::Csa, &mut p, 0, 0).unwrap();
        let text = String::from_utf8(result).unwrap();
        assert!(
            text.contains("/CIEBased"),
            "CSA resource should contain CIEBased"
        );
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn color_resource_crd_dispatches() {
        let mut p = roundtrip(&mut Profile::new_srgb());
        let result =
            get_postscript_color_resource(PostScriptResourceType::Crd, &mut p, 0, 0).unwrap();
        let text = String::from_utf8(result).unwrap();
        assert!(
            text.contains("/ColorRenderingType"),
            "CRD resource should contain ColorRenderingType"
        );
    }
}
