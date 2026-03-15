use std::ops::{Mul, Sub};

#[allow(dead_code)]
const MATRIX_DET_TOLERANCE: f64 = 0.0001;
#[allow(dead_code)]
const CLOSE_ENOUGH_TOLERANCE: f64 = 1.0 / 65535.0;

/// 3-component vector.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec3(pub [f64; 3]);

/// 3×3 matrix, row-major: `Mat3.0[row].0[col]`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mat3(pub [Vec3; 3]);

impl Vec3 {
    pub fn new(_x: f64, _y: f64, _z: f64) -> Self {
        todo!()
    }

    pub fn cross(&self, _other: &Vec3) -> Vec3 {
        todo!()
    }

    pub fn dot(&self, _other: &Vec3) -> f64 {
        todo!()
    }

    pub fn length(&self) -> f64 {
        todo!()
    }

    pub fn distance(&self, _other: &Vec3) -> f64 {
        todo!()
    }
}

impl Sub for Vec3 {
    type Output = Vec3;
    fn sub(self, _rhs: Vec3) -> Vec3 {
        todo!()
    }
}

impl Mat3 {
    pub fn identity() -> Self {
        todo!()
    }

    pub fn is_identity(&self) -> bool {
        todo!()
    }

    pub fn inverse(&self) -> Option<Mat3> {
        todo!()
    }

    pub fn solve(&self, _b: &Vec3) -> Option<Vec3> {
        todo!()
    }

    pub fn eval(&self, _v: &Vec3) -> Vec3 {
        todo!()
    }
}

impl Mul for Mat3 {
    type Output = Mat3;
    fn mul(self, _rhs: Mat3) -> Mat3 {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f64 = 1e-10;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < EPSILON
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn vec3_new() {
        let v = Vec3::new(1.0, 2.0, 3.0);
        assert_eq!(v.0, [1.0, 2.0, 3.0]);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn vec3_sub() {
        let a = Vec3::new(3.0, 5.0, 7.0);
        let b = Vec3::new(1.0, 2.0, 3.0);
        let r = a - b;
        assert_eq!(r.0, [2.0, 3.0, 4.0]);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn vec3_dot() {
        let a = Vec3::new(1.0, 2.0, 3.0);
        let b = Vec3::new(4.0, 5.0, 6.0);
        assert!(close(a.dot(&b), 32.0)); // 1*4+2*5+3*6 = 32
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn vec3_cross() {
        let a = Vec3::new(1.0, 0.0, 0.0);
        let b = Vec3::new(0.0, 1.0, 0.0);
        let c = a.cross(&b);
        assert!(close(c.0[0], 0.0));
        assert!(close(c.0[1], 0.0));
        assert!(close(c.0[2], 1.0));
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn vec3_length() {
        let v = Vec3::new(3.0, 4.0, 0.0);
        assert!(close(v.length(), 5.0));
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn vec3_distance() {
        let a = Vec3::new(1.0, 2.0, 3.0);
        let b = Vec3::new(4.0, 6.0, 3.0);
        assert!(close(a.distance(&b), 5.0)); // sqrt(9+16+0)
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn mat3_identity_is_identity() {
        let id = Mat3::identity();
        assert!(id.is_identity());
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn mat3_mul_identity() {
        let m = Mat3([
            Vec3::new(1.0, 2.0, 3.0),
            Vec3::new(4.0, 5.0, 6.0),
            Vec3::new(7.0, 8.0, 10.0),
        ]);
        let id = Mat3::identity();
        let r = m * id;
        assert_eq!(r, m);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn mat3_inverse_times_original_is_identity() {
        let m = Mat3([
            Vec3::new(1.0, 2.0, 3.0),
            Vec3::new(0.0, 1.0, 4.0),
            Vec3::new(5.0, 6.0, 0.0),
        ]);
        let inv = m.inverse().expect("should be invertible");
        let product = m * inv;
        assert!(product.is_identity());
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn mat3_singular_inverse_returns_none() {
        let m = Mat3([
            Vec3::new(1.0, 2.0, 3.0),
            Vec3::new(4.0, 5.0, 6.0),
            Vec3::new(7.0, 8.0, 9.0),
        ]);
        assert!(m.inverse().is_none());
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn mat3_eval() {
        let m = Mat3([
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 2.0, 0.0),
            Vec3::new(0.0, 0.0, 3.0),
        ]);
        let v = Vec3::new(1.0, 1.0, 1.0);
        let r = m.eval(&v);
        assert!(close(r.0[0], 1.0));
        assert!(close(r.0[1], 2.0));
        assert!(close(r.0[2], 3.0));
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn mat3_solve() {
        // Solve Ax = b where A = [[2,1,0],[0,3,1],[0,0,4]], b = [4,7,8]
        // Expected: x = [0.5, 1.0, 2.0]
        let a = Mat3([
            Vec3::new(2.0, 1.0, 0.0),
            Vec3::new(0.0, 3.0, 1.0),
            Vec3::new(0.0, 0.0, 4.0),
        ]);
        let b = Vec3::new(4.0, 7.0, 8.0);
        let x = a.solve(&b).expect("should be solvable");
        assert!(close(x.0[0], 7.0 / 6.0));
        assert!(close(x.0[1], 1.0 + 1.0 / 3.0));
        assert!(close(x.0[2], 2.0));
    }
}
