// ============================================================================
// Gamut boundary description (C版: cmssm.c)
// ============================================================================
//
// Jan Morovic's Segment Maxima method.
// Models device gamut using spherical coordinates centered at Lab(50, 0, 0).

use crate::math::mtrx::Vec3;
use crate::types::CieLab;

const SECTORS: usize = 16;

// ============================================================================
// Spherical coordinates
// ============================================================================

/// Spherical coordinate (r, alpha=hue, theta=lightness angle).
#[derive(Clone, Copy, Default)]
struct Spherical {
    r: f64,
    alpha: f64, // hue angle in degrees [0, 360)
    theta: f64, // lightness angle in degrees [0, 180]
}

/// Convert Lab (centered at L*=50) to spherical coordinates.
fn to_spherical(lab: &CieLab) -> Spherical {
    // Center at Lab(50, 0, 0)
    let l = lab.l - 50.0;
    let a = lab.a;
    let b = lab.b;

    let r = (l * l + a * a + b * b).sqrt();
    let alpha = atan2_positive(a, b);
    let theta = atan2_positive((a * a + b * b).sqrt(), l);

    Spherical { r, alpha, theta }
}

/// Convert spherical coordinates back to Lab (centered at L*=50).
fn to_cartesian(sp: &Spherical) -> CieLab {
    let sin_theta = sp.theta.to_radians().sin();
    let cos_theta = sp.theta.to_radians().cos();
    let sin_alpha = sp.alpha.to_radians().sin();
    let cos_alpha = sp.alpha.to_radians().cos();

    CieLab {
        l: sp.r * cos_theta + 50.0,
        a: sp.r * sin_theta * sin_alpha,
        b: sp.r * sin_theta * cos_alpha,
    }
}

/// atan2 returning degrees in [0, 360).
fn atan2_positive(y: f64, x: f64) -> f64 {
    let mut a = y.atan2(x).to_degrees();
    if a < 0.0 {
        a += 360.0;
    }
    a
}

/// Quantize spherical coordinates to grid indices.
fn quantize(sp: &Spherical) -> (usize, usize) {
    let alpha = ((sp.alpha * SECTORS as f64) / 360.0).floor() as usize;
    let theta = ((sp.theta * SECTORS as f64) / 180.0).floor() as usize;
    (alpha.min(SECTORS - 1), theta.min(SECTORS - 1))
}

// ============================================================================
// Parametric line & closest point
// ============================================================================

/// Parametric line: P(t) = a + t * u
struct Line {
    a: Vec3,
    u: Vec3,
}

fn line_of_2_points(a: &Vec3, b: &Vec3) -> Line {
    Line { a: *a, u: *b - *a }
}

fn point_on_line(line: &Line, t: f64) -> Vec3 {
    Vec3::new(
        line.a.0[0] + t * line.u.0[0],
        line.a.0[1] + t * line.u.0[1],
        line.a.0[2] + t * line.u.0[2],
    )
}

/// Find closest point on ray (line1) to edge segment (line2).
/// line1 is treated as a ray (t >= 0, no upper bound).
/// line2 is treated as a finite segment (t clamped to [0, 1]).
/// Returns the point on line1 (the ray).
/// Based on the algorithm from softSurfer.
fn closest_ray_to_segment(ray: &Line, segment: &Line) -> Vec3 {
    let dp = ray.a - segment.a;

    let a_val = ray.u.dot(&ray.u);
    let b_val = ray.u.dot(&segment.u);
    let c_val = segment.u.dot(&segment.u);
    let d_val = ray.u.dot(&dp);
    let e_val = segment.u.dot(&dp);

    let denom = a_val * c_val - b_val * b_val;

    if denom.abs() < 1e-12 {
        // Lines are parallel; return ray origin
        return ray.a;
    }

    // Ray parameter: clamp to [0, ∞) (forward direction only)
    let sc = ((b_val * e_val - c_val * d_val) / denom).max(0.0);

    // Segment parameter: clamp to [0, 1]
    let _tc = ((a_val * e_val - b_val * d_val) / denom).clamp(0.0, 1.0);

    point_on_line(ray, sc)
}

// ============================================================================
// Grid point
// ============================================================================

#[derive(Clone, Copy, PartialEq)]
enum PointType {
    Empty,
    Specified,
    Modeled,
}

#[derive(Clone, Copy)]
struct GdbPoint {
    kind: PointType,
    p: Spherical,
}

impl Default for GdbPoint {
    fn default() -> Self {
        Self {
            kind: PointType::Empty,
            p: Spherical::default(),
        }
    }
}

// ============================================================================
// GamutBoundary
// ============================================================================

/// Gamut boundary descriptor using Segment Maxima method.
/// C版: `cmsGDB`
pub struct GamutBoundary {
    grid: [[GdbPoint; SECTORS]; SECTORS],
}

impl GamutBoundary {
    /// Create a new empty gamut boundary descriptor.
    pub fn new() -> Self {
        Self {
            grid: [[GdbPoint::default(); SECTORS]; SECTORS],
        }
    }

    /// Add a Lab sample point to the gamut boundary.
    /// Keeps only the maximum-radius point per sector.
    pub fn add_point(&mut self, lab: &CieLab) -> bool {
        let sp = to_spherical(lab);
        let (ai, ti) = quantize(&sp);

        let pt = &mut self.grid[ti][ai];
        if pt.kind == PointType::Empty || sp.r > pt.p.r {
            pt.kind = PointType::Specified;
            pt.p = sp;
        }
        true
    }

    /// Interpolate missing sectors from neighboring sample data.
    pub fn compute(&mut self) {
        // Black plane (theta = 0)
        for ai in 0..SECTORS {
            self.interpolate_missing(ai, 0);
        }
        // White plane (theta = SECTORS-1)
        for ai in 0..SECTORS {
            self.interpolate_missing(ai, SECTORS - 1);
        }
        // Mid-tones
        for ti in 1..(SECTORS - 1) {
            for ai in 0..SECTORS {
                self.interpolate_missing(ai, ti);
            }
        }
    }

    /// Check if a Lab point falls within the modeled gamut boundary.
    pub fn check(&self, lab: &CieLab) -> bool {
        let sp = to_spherical(lab);
        let (ai, ti) = quantize(&sp);

        let pt = &self.grid[ti][ai];
        if pt.kind == PointType::Empty {
            return false;
        }
        sp.r <= pt.p.r
    }

    /// Find non-empty neighbors around a grid sector (spiral search).
    fn find_near_sectors(&self, ai: usize, ti: usize) -> Vec<Spherical> {
        let mut result = Vec::new();

        // Spiral offsets: immediate 8-neighbors, then 16-ring
        let offsets: [(i32, i32); 24] = [
            // Ring 1 (8)
            (1, 0),
            (1, 1),
            (0, 1),
            (-1, 1),
            (-1, 0),
            (-1, -1),
            (0, -1),
            (1, -1),
            // Ring 2 (16)
            (2, 0),
            (2, 1),
            (2, 2),
            (1, 2),
            (0, 2),
            (-1, 2),
            (-2, 2),
            (-2, 1),
            (-2, 0),
            (-2, -1),
            (-2, -2),
            (-1, -2),
            (0, -2),
            (1, -2),
            (2, -2),
            (2, -1),
        ];

        for &(da, dt) in &offsets {
            // Wrap alpha (hue is circular)
            let na = ((ai as i32 + da).rem_euclid(SECTORS as i32)) as usize;
            let nt = ti as i32 + dt;
            if nt < 0 || nt >= SECTORS as i32 {
                continue;
            }
            let nt = nt as usize;

            let pt = &self.grid[nt][na];
            if pt.kind != PointType::Empty {
                result.push(pt.p);
            }
        }
        result
    }

    /// Interpolate a missing sector from its neighbors.
    fn interpolate_missing(&mut self, ai: usize, ti: usize) {
        if self.grid[ti][ai].kind != PointType::Empty {
            return;
        }

        let neighbors = self.find_near_sectors(ai, ti);
        if neighbors.len() < 2 {
            return;
        }

        // Create ray from center through sector center
        let sector_sp = Spherical {
            r: 50.0,
            alpha: (ai as f64 + 0.5) * 360.0 / SECTORS as f64,
            theta: (ti as f64 + 0.5) * 180.0 / SECTORS as f64,
        };
        let sector_lab = to_cartesian(&sector_sp);
        let center = Vec3::new(0.0, 0.0, 0.0); // Lab(50,0,0) centered
        let target = Vec3::new(sector_lab.l - 50.0, sector_lab.a, sector_lab.b);
        let ray = line_of_2_points(&center, &target);

        let mut best_r = 0.0;

        // For each pair of neighboring points, find intersection with ray
        for i in 0..neighbors.len() {
            let j = (i + 1) % neighbors.len();

            let lab_i = to_cartesian(&neighbors[i]);
            let lab_j = to_cartesian(&neighbors[j]);

            let p1 = Vec3::new(lab_i.l - 50.0, lab_i.a, lab_i.b);
            let p2 = Vec3::new(lab_j.l - 50.0, lab_j.a, lab_j.b);

            let edge = line_of_2_points(&p1, &p2);
            let closest = closest_ray_to_segment(&ray, &edge);

            let r = closest.length();
            if r > best_r {
                best_r = r;
            }
        }

        if best_r > 0.0 {
            self.grid[ti][ai] = GdbPoint {
                kind: PointType::Modeled,
                p: Spherical {
                    r: best_r,
                    alpha: sector_sp.alpha,
                    theta: sector_sp.theta,
                },
            };
        }
    }
}

impl Default for GamutBoundary {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spherical_round_trip() {
        let lab = CieLab {
            l: 70.0,
            a: 30.0,
            b: -20.0,
        };
        let sp = to_spherical(&lab);
        let lab2 = to_cartesian(&sp);
        assert!((lab.l - lab2.l).abs() < 1e-10);
        assert!((lab.a - lab2.a).abs() < 1e-10);
        assert!((lab.b - lab2.b).abs() < 1e-10);
    }

    #[test]
    fn spherical_center_is_zero_radius() {
        let lab = CieLab {
            l: 50.0,
            a: 0.0,
            b: 0.0,
        };
        let sp = to_spherical(&lab);
        assert!(sp.r < 1e-10);
    }

    #[test]
    fn empty_gbd_check_returns_false() {
        let gbd = GamutBoundary::new();
        let lab = CieLab {
            l: 50.0,
            a: 20.0,
            b: 20.0,
        };
        assert!(!gbd.check(&lab));
    }

    #[test]
    fn add_point_and_check_inside() {
        let mut gbd = GamutBoundary::new();

        // Add a point with large chroma
        gbd.add_point(&CieLab {
            l: 60.0,
            a: 50.0,
            b: 30.0,
        });

        // A point in the same sector with smaller chroma should be inside
        assert!(gbd.check(&CieLab {
            l: 55.0,
            a: 25.0,
            b: 15.0,
        }));
    }

    #[test]
    fn add_point_and_check_outside() {
        let mut gbd = GamutBoundary::new();

        // Add a point with moderate chroma
        gbd.add_point(&CieLab {
            l: 60.0,
            a: 20.0,
            b: 10.0,
        });

        // A point with larger chroma in the same direction should be outside
        assert!(!gbd.check(&CieLab {
            l: 60.0,
            a: 80.0,
            b: 40.0,
        }));
    }

    #[test]
    fn compute_fills_neighboring_sectors() {
        let mut gbd = GamutBoundary::new();

        // Add points across multiple hue angles
        for angle_deg in (0..360).step_by(30) {
            let rad = (angle_deg as f64).to_radians();
            gbd.add_point(&CieLab {
                l: 60.0,
                a: 40.0 * rad.sin(),
                b: 40.0 * rad.cos(),
            });
        }

        gbd.compute();

        // After compute, a point at an intermediate hue should be checkable
        let mid = CieLab {
            l: 60.0,
            a: 20.0 * 15.0_f64.to_radians().sin(),
            b: 20.0 * 15.0_f64.to_radians().cos(),
        };
        // Point should be inside (smaller chroma than boundary)
        assert!(
            gbd.check(&mid),
            "computed GBD should cover intermediate hue"
        );
    }
}
