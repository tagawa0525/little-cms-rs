//! Pipeline and Stage (LUT) engine.
//!
//! C版対応: `cmslut.c`
//!
//! Provides the Pipeline/Stage system that is the core of color transformation.
//! A Pipeline is a chain of Stages, each performing one step of color processing
//! (curves, matrices, CLUTs, etc.).

use crate::curves::gamma::ToneCurve;
use crate::types::StageSignature;

/// Stage data payload.
#[derive(Clone)]
pub enum StageData {
    Curves(Vec<ToneCurve>),
    Matrix {
        coefficients: Vec<f64>,
        offset: Option<Vec<f64>>,
    },
    None,
}

/// A single processing element in a pipeline.
#[derive(Clone)]
#[allow(dead_code)]
pub struct Stage {
    stage_type: StageSignature,
    implements: StageSignature,
    input_channels: u32,
    output_channels: u32,
    data: StageData,
}

impl Stage {
    pub fn stage_type(&self) -> StageSignature {
        self.stage_type
    }

    pub fn input_channels(&self) -> u32 {
        self.input_channels
    }

    pub fn output_channels(&self) -> u32 {
        self.output_channels
    }

    pub fn data(&self) -> &StageData {
        &self.data
    }

    pub fn new_identity(_n: u32) -> Option<Self> {
        todo!()
    }

    pub fn new_tone_curves(_curves: Option<&[ToneCurve]>, _n: u32) -> Option<Self> {
        todo!()
    }

    pub fn new_matrix(
        _rows: u32,
        _cols: u32,
        _matrix: &[f64],
        _offset: Option<&[f64]>,
    ) -> Option<Self> {
        todo!()
    }

    pub fn eval(&self, _input: &[f32], _output: &mut [f32]) {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Stage: Identity
    // ========================================================================

    #[test]
    #[ignore = "not yet implemented"]
    fn stage_identity_passthrough() {
        let stage = Stage::new_identity(3).unwrap();
        let input = [0.25f32, 0.5, 0.75];
        let mut output = [0.0f32; 3];
        stage.eval(&input, &mut output);
        assert_eq!(output, input);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn stage_identity_channel_count() {
        let stage = Stage::new_identity(4).unwrap();
        assert_eq!(stage.input_channels(), 4);
        assert_eq!(stage.output_channels(), 4);
        assert_eq!(stage.stage_type(), StageSignature::IdentityElem);
    }

    // ========================================================================
    // Stage: Curves
    // ========================================================================

    #[test]
    #[ignore = "not yet implemented"]
    fn stage_curves_gamma() {
        let curve = ToneCurve::build_gamma(2.2).unwrap();
        let stage =
            Stage::new_tone_curves(Some(&[curve.clone(), curve.clone(), curve]), 3).unwrap();
        assert_eq!(stage.stage_type(), StageSignature::CurveSetElem);
        assert_eq!(stage.input_channels(), 3);
        assert_eq!(stage.output_channels(), 3);

        let input = [0.5f32, 0.5, 0.5];
        let mut output = [0.0f32; 3];
        stage.eval(&input, &mut output);

        let expected = 0.5f64.powf(2.2) as f32;
        for &v in &output {
            assert!((v - expected).abs() < 0.01, "got {v}, expected {expected}");
        }
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn stage_curves_per_channel() {
        let c1 = ToneCurve::build_gamma(1.0).unwrap();
        let c2 = ToneCurve::build_gamma(2.0).unwrap();
        let c3 = ToneCurve::build_gamma(3.0).unwrap();
        let stage = Stage::new_tone_curves(Some(&[c1, c2, c3]), 3).unwrap();

        let input = [0.5f32, 0.5, 0.5];
        let mut output = [0.0f32; 3];
        stage.eval(&input, &mut output);

        assert!((output[0] - 0.5).abs() < 0.01);
        assert!((output[1] - 0.25).abs() < 0.01);
        assert!((output[2] - 0.125).abs() < 0.01);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn stage_curves_none_is_identity() {
        let stage = Stage::new_tone_curves(None, 3).unwrap();
        let input = [0.3f32, 0.6, 0.9];
        let mut output = [0.0f32; 3];
        stage.eval(&input, &mut output);

        for i in 0..3 {
            assert!(
                (output[i] - input[i]).abs() < 0.001,
                "ch {i}: got {}, expected {}",
                output[i],
                input[i]
            );
        }
    }

    // ========================================================================
    // Stage: Matrix
    // ========================================================================

    #[test]
    #[ignore = "not yet implemented"]
    fn stage_matrix_identity() {
        let matrix = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
        let stage = Stage::new_matrix(3, 3, &matrix, None).unwrap();
        assert_eq!(stage.stage_type(), StageSignature::MatrixElem);
        assert_eq!(stage.input_channels(), 3);
        assert_eq!(stage.output_channels(), 3);

        let input = [0.2f32, 0.4, 0.6];
        let mut output = [0.0f32; 3];
        stage.eval(&input, &mut output);

        for i in 0..3 {
            assert!(
                (output[i] - input[i]).abs() < 1e-6,
                "ch {i}: got {}, expected {}",
                output[i],
                input[i]
            );
        }
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn stage_matrix_scale() {
        let matrix = [2.0, 0.0, 0.0, 0.0, 3.0, 0.0, 0.0, 0.0, 4.0];
        let stage = Stage::new_matrix(3, 3, &matrix, None).unwrap();

        let input = [0.1f32, 0.2, 0.3];
        let mut output = [0.0f32; 3];
        stage.eval(&input, &mut output);

        assert!((output[0] - 0.2).abs() < 1e-6);
        assert!((output[1] - 0.6).abs() < 1e-6);
        assert!((output[2] - 1.2).abs() < 1e-5);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn stage_matrix_with_offset() {
        let matrix = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
        let offset = [0.1, 0.2, 0.3];
        let stage = Stage::new_matrix(3, 3, &matrix, Some(&offset)).unwrap();

        let input = [0.0f32; 3];
        let mut output = [0.0f32; 3];
        stage.eval(&input, &mut output);

        assert!((output[0] - 0.1).abs() < 1e-6);
        assert!((output[1] - 0.2).abs() < 1e-6);
        assert!((output[2] - 0.3).abs() < 1e-6);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn stage_matrix_invalid_dims() {
        assert!(Stage::new_matrix(0, 3, &[], None).is_none());
        assert!(Stage::new_matrix(3, 0, &[], None).is_none());
    }

    // ========================================================================
    // Stage: Clone
    // ========================================================================

    #[test]
    #[ignore = "not yet implemented"]
    fn stage_clone() {
        let curve = ToneCurve::build_gamma(2.2).unwrap();
        let stage =
            Stage::new_tone_curves(Some(&[curve.clone(), curve.clone(), curve]), 3).unwrap();
        let cloned = stage.clone();

        let input = [0.5f32, 0.5, 0.5];
        let mut out1 = [0.0f32; 3];
        let mut out2 = [0.0f32; 3];
        stage.eval(&input, &mut out1);
        cloned.eval(&input, &mut out2);

        assert_eq!(out1, out2);
        assert_eq!(cloned.stage_type(), stage.stage_type());
        assert_eq!(cloned.input_channels(), stage.input_channels());
    }
}
