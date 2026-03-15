use std::ops::{Mul, Sub};

const MATRIX_DET_TOLERANCE: f64 = 0.0001;
const CLOSE_ENOUGH_TOLERANCE: f64 = 1.0 / 65535.0;

/// 3-component vector.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec3(pub [f64; 3]);

/// 3×3 matrix, row-major: `Mat3.0[row].0[col]`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mat3(pub [Vec3; 3]);

impl Vec3 {
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self([x, y, z])
    }

    pub fn cross(&self, other: &Vec3) -> Vec3 {
        Vec3([
            self.0[1] * other.0[2] - self.0[2] * other.0[1],
            self.0[2] * other.0[0] - self.0[0] * other.0[2],
            self.0[0] * other.0[1] - self.0[1] * other.0[0],
        ])
    }

    pub fn dot(&self, other: &Vec3) -> f64 {
        self.0[0] * other.0[0] + self.0[1] * other.0[1] + self.0[2] * other.0[2]
    }

    pub fn length(&self) -> f64 {
        self.dot(self).sqrt()
    }

    pub fn distance(&self, other: &Vec3) -> f64 {
        (*self - *other).length()
    }
}

impl Sub for Vec3 {
    type Output = Vec3;
    fn sub(self, rhs: Vec3) -> Vec3 {
        Vec3([
            self.0[0] - rhs.0[0],
            self.0[1] - rhs.0[1],
            self.0[2] - rhs.0[2],
        ])
    }
}

impl Mat3 {
    pub fn identity() -> Self {
        Mat3([
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
        ])
    }

    pub fn is_identity(&self) -> bool {
        let id = Mat3::identity();
        for i in 0..3 {
            for j in 0..3 {
                if (self.0[i].0[j] - id.0[i].0[j]).abs() > CLOSE_ENOUGH_TOLERANCE {
                    return false;
                }
            }
        }
        true
    }

    pub fn inverse(&self) -> Option<Mat3> {
        // Cofactor expansion
        let a = &self.0;
        let c00 = a[1].0[1] * a[2].0[2] - a[1].0[2] * a[2].0[1];
        let c01 = -(a[1].0[0] * a[2].0[2] - a[1].0[2] * a[2].0[0]);
        let c02 = a[1].0[0] * a[2].0[1] - a[1].0[1] * a[2].0[0];

        let det = a[0].0[0] * c00 + a[0].0[1] * c01 + a[0].0[2] * c02;
        if det.abs() < MATRIX_DET_TOLERANCE {
            return None;
        }

        let c10 = -(a[0].0[1] * a[2].0[2] - a[0].0[2] * a[2].0[1]);
        let c11 = a[0].0[0] * a[2].0[2] - a[0].0[2] * a[2].0[0];
        let c12 = -(a[0].0[0] * a[2].0[1] - a[0].0[1] * a[2].0[0]);
        let c20 = a[0].0[1] * a[1].0[2] - a[0].0[2] * a[1].0[1];
        let c21 = -(a[0].0[0] * a[1].0[2] - a[0].0[2] * a[1].0[0]);
        let c22 = a[0].0[0] * a[1].0[1] - a[0].0[1] * a[1].0[0];

        let inv_det = 1.0 / det;
        Some(Mat3([
            Vec3::new(c00 * inv_det, c10 * inv_det, c20 * inv_det),
            Vec3::new(c01 * inv_det, c11 * inv_det, c21 * inv_det),
            Vec3::new(c02 * inv_det, c12 * inv_det, c22 * inv_det),
        ]))
    }

    pub fn solve(&self, b: &Vec3) -> Option<Vec3> {
        let inv = self.inverse()?;
        Some(inv.eval(b))
    }

    pub fn eval(&self, v: &Vec3) -> Vec3 {
        Vec3([self.0[0].dot(v), self.0[1].dot(v), self.0[2].dot(v)])
    }
}

impl Mul for Mat3 {
    type Output = Mat3;
    fn mul(self, rhs: Mat3) -> Mat3 {
        let mut result = [[0.0f64; 3]; 3];
        for (i, row) in result.iter_mut().enumerate() {
            for (j, cell) in row.iter_mut().enumerate() {
                for k in 0..3 {
                    *cell += self.0[i].0[k] * rhs.0[k].0[j];
                }
            }
        }
        Mat3([Vec3(result[0]), Vec3(result[1]), Vec3(result[2])])
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

    fn vec3_new() {
        let v = Vec3::new(1.0, 2.0, 3.0);
        assert_eq!(v.0, [1.0, 2.0, 3.0]);
    }

    #[test]

    fn vec3_sub() {
        let a = Vec3::new(3.0, 5.0, 7.0);
        let b = Vec3::new(1.0, 2.0, 3.0);
        let r = a - b;
        assert_eq!(r.0, [2.0, 3.0, 4.0]);
    }

    #[test]

    fn vec3_dot() {
        let a = Vec3::new(1.0, 2.0, 3.0);
        let b = Vec3::new(4.0, 5.0, 6.0);
        assert!(close(a.dot(&b), 32.0)); // 1*4+2*5+3*6 = 32
    }

    #[test]

    fn vec3_cross() {
        let a = Vec3::new(1.0, 0.0, 0.0);
        let b = Vec3::new(0.0, 1.0, 0.0);
        let c = a.cross(&b);
        assert!(close(c.0[0], 0.0));
        assert!(close(c.0[1], 0.0));
        assert!(close(c.0[2], 1.0));
    }

    #[test]

    fn vec3_length() {
        let v = Vec3::new(3.0, 4.0, 0.0);
        assert!(close(v.length(), 5.0));
    }

    #[test]

    fn vec3_distance() {
        let a = Vec3::new(1.0, 2.0, 3.0);
        let b = Vec3::new(4.0, 6.0, 3.0);
        assert!(close(a.distance(&b), 5.0)); // sqrt(9+16+0)
    }

    #[test]

    fn mat3_identity_is_identity() {
        let id = Mat3::identity();
        assert!(id.is_identity());
    }

    #[test]

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

    fn mat3_singular_inverse_returns_none() {
        let m = Mat3([
            Vec3::new(1.0, 2.0, 3.0),
            Vec3::new(4.0, 5.0, 6.0),
            Vec3::new(7.0, 8.0, 9.0),
        ]);
        assert!(m.inverse().is_none());
    }

    #[test]

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

    fn mat3_solve() {
        // Solve Ax = b where A = [[2,1,0],[0,3,1],[0,0,4]], b = [4,7,8]
        // Expected: x = [7/6, 5/3, 2]
        let a = Mat3([
            Vec3::new(2.0, 1.0, 0.0),
            Vec3::new(0.0, 3.0, 1.0),
            Vec3::new(0.0, 0.0, 4.0),
        ]);
        let b = Vec3::new(4.0, 7.0, 8.0);
        let x = a.solve(&b).expect("should be solvable");
        assert!(close(x.0[0], 7.0 / 6.0));
        assert!(close(x.0[1], 5.0 / 3.0));
        assert!(close(x.0[2], 2.0));
    }
}
