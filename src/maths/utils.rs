//! Complex numbers
//!
//!
//! Examples
//!

// Stores a complex number consisting of a real and an imaginary component
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Complex {
    pub real: f64,
    pub imag: f64,
}
impl Complex {
    /// Returns a complex number with zero real component and zero imaginary component
    pub const fn zero() -> Self {
        return Self {
            real: 0.0,
            imag: 0.0,
        };
    }
    /// Returns a complex number from the given real and imaginary components
    pub fn new(real: f64, imag: f64) -> Self {
        return Self { real, imag };
    }

    pub fn abs(&self) -> f64 {
        return (self.real.powi(2) + self.imag.powi(2)).sqrt();
    }
    /// Computes the angle from the origin
    pub fn phase(self) -> f64 {
        return self.imag.atan2(self.real);
    }
}
macro_rules! impl_scalar_ops {
    ($($t:ty),* $(,)?) => { $(
        // z * 2
        impl core::ops::Mul<$t> for Complex {
            type Output = Complex;
            fn mul(self, rhs: $t) -> Complex {
                Complex {real: self.real * rhs as f64, imag: self.imag * rhs as f64}
            }
        }
        // 2 * z
        impl core::ops::Mul<Complex> for $t {
            type Output = Complex;
            fn mul(self, rhs: Complex) -> Complex {
                rhs * self
            }
        }
        // z *= 2
        impl core::ops::MulAssign<$t> for Complex {
            fn mul_assign(&mut self, rhs: $t) {
                *self = *self * rhs;
            }
        }
        // z / 2
        impl core::ops::Div<$t> for Complex {
            type Output = Complex;
            fn div(self, rhs: $t) -> Complex {
                Complex { real: self.real / rhs as f64, imag: self.imag / rhs as f64 }
            }
        }
        // z /= 2
        impl core::ops::DivAssign<$t> for Complex {
            fn div_assign(&mut self, rhs: $t) {
                *self = *self / rhs;
            }
        }
        // z + 2
        impl core::ops::Add<$t> for Complex {
            type Output = Complex;
            fn add(self, rhs: $t) -> Complex {
                Complex {real: self.real + rhs as f64, imag: self.imag + rhs as f64}
            }
        }
        // 2 + z
        impl core::ops::Add<Complex> for $t {
            type Output = Complex;
            fn add(self, rhs: Complex) -> Complex {
                rhs + self
            }
        }
        // z += 2
        impl core::ops::AddAssign<$t> for Complex {
            fn add_assign(&mut self, rhs: $t) {
                *self = *self + rhs;
            }
        }
        // z - 2
        impl core::ops::Sub<$t> for Complex {
            type Output = Complex;
            fn sub(self, rhs: $t) -> Complex {
                Complex {real: self.real - rhs as f64, imag: self.imag - rhs as f64}
            }
        }
        // 2 - z
        impl core::ops::Sub<Complex> for $t {
            type Output = Complex;
            fn sub(self, rhs: Complex) -> Complex {
                rhs - self
            }
        }
        // z -= 2
        impl core::ops::SubAssign<$t> for Complex {
            fn sub_assign(&mut self, rhs: $t) {
                *self = *self - rhs;
            }
        }

        impl From<$t> for Complex {
            fn from(value: $t) -> Self {
                Self {
                    real: value as f64,
                    imag: 0.0
                }
            }
        }


    )*


    };
}

// Generate trait impls for arithmetic operators for all basic types
impl_scalar_ops!(
    u8, u16, u32, u64, u128, usize, f32, f64, i8, i16, i32, i64, i128, isize,
);

impl core::ops::Add<Complex> for Complex {
    type Output = Complex;
    fn add(self, rhs: Complex) -> Self::Output {
        return Self {
            real: self.real + rhs.real,
            imag: self.imag + rhs.imag,
        };
    }
}

impl core::ops::AddAssign<Complex> for Complex {
    fn add_assign(&mut self, rhs: Complex) {
        self.real += rhs.real;
        self.imag += rhs.imag;
    }
}

impl core::ops::Sub<Complex> for Complex {
    type Output = Complex;
    fn sub(self, rhs: Complex) -> Self::Output {
        return Self {
            real: self.real - rhs.real,
            imag: self.imag - rhs.imag,
        };
    }
}

impl core::ops::SubAssign<Complex> for Complex {
    fn sub_assign(&mut self, rhs: Complex) {
        self.real -= rhs.real;
        self.imag -= rhs.imag;
    }
}

impl core::ops::Mul<Complex> for Complex {
    type Output = Complex;
    /// Performs complex multiplication
    /// e.g. (a+bi)(c+di) = (ac - bd) + (ad+bc)i
    fn mul(self, rhs: Self) -> Self::Output {
        return Self {
            real: self.real * rhs.real - self.imag * rhs.imag,
            imag: self.real * rhs.imag + self.imag * rhs.real,
        };
    }
}

impl core::ops::MulAssign<Complex> for Complex {
    fn mul_assign(&mut self, rhs: Complex) {
        self.real *= rhs.real;
        self.imag *= rhs.imag;
    }
}

impl From<(f64, f64)> for Complex {
    fn from(value: (f64, f64)) -> Self {
        Self {
            real: value.0,
            imag: value.1,
        }
    }
}

pub const I: Complex = Complex {
    real: 0.0,
    imag: 1.0,
};

#[cfg(test)]
mod tests {
    use super::*;
    use core::f64::consts::PI;

    /// Minimum distance to floats should be
    const EPS: f64 = 1e-12;

    /// Asserts two floats are within `EPS` of each other.
    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < EPS,
            "expected {expected}, got {actual}"
        );
    }

    /// Asserts both components of two complex numbers are within `EPS`.
    fn assert_complex_close(actual: Complex, expected: Complex) {
        assert!(
            (actual.real - expected.real).abs() < EPS && (actual.imag - expected.imag).abs() < EPS,
            "expected {expected:?}, got {actual:?}"
        );
    }
    #[test]
    fn basic_usage() {
        let a = 3 + 4 * I;
        let b = 3 + 2 * I;
        assert_eq!(a + b, Complex::new(6.0, 6.0))
    }

    #[test]
    fn new_sets_components() {
        let result = Complex::new(5.0, 2.4);
        assert_eq!(result.real, 5.0);
        assert_eq!(result.imag, 2.4);
    }

    #[test]
    fn zero_is_zero() {
        assert_eq!(Complex::zero(), Complex::new(0.0, 0.0));
    }

    #[test]
    fn i_constant() {
        assert_eq!(I, Complex::new(0.0, 1.0));
    }

    #[test]
    fn from_f64_has_zero_imaginary_part() {
        assert_eq!(Complex::from(3.5), Complex::new(3.5, 0.0));
    }

    #[test]
    fn from_tuple() {
        assert_eq!(Complex::from((1.0, -2.0)), Complex::new(1.0, -2.0));
    }

    // ---- abs ----

    #[test]
    fn abs_pythagorean_triple() {
        assert_eq!(Complex::new(3.0, 4.0).abs(), 5.0);
    }

    #[test]
    fn abs_ignores_signs() {
        assert_eq!(Complex::new(-3.0, -4.0).abs(), 5.0);
        assert_eq!(Complex::new(-3.0, 4.0).abs(), 5.0);
    }

    #[test]
    fn abs_of_zero_is_zero() {
        assert_eq!(Complex::zero().abs(), 0.0);
    }

    // ---- phase ----

    #[test]
    fn phase_on_each_axis() {
        assert_close(Complex::new(1.0, 0.0).phase(), 0.0);
        assert_close(Complex::new(0.0, 1.0).phase(), PI / 2.0);
        assert_close(Complex::new(-1.0, 0.0).phase(), PI);
        assert_close(Complex::new(0.0, -1.0).phase(), -PI / 2.0);
    }

    #[test]
    fn phase_is_independent_of_magnitude() {
        assert_close(
            Complex::new(1.0, 2.0).phase(),
            Complex::new(100.0, 200.0).phase(),
        );
    }

    // ---- addition / subtraction ----

    #[test]
    fn add() {
        let result = Complex::new(1.0, 2.0) + Complex::new(3.0, -5.0);
        assert_eq!(result, Complex::new(4.0, -3.0));
    }

    #[test]
    fn add_assign() {
        let mut z = Complex::new(1.0, 2.0);
        z += Complex::new(0.5, 0.5);
        assert_eq!(z, Complex::new(1.5, 2.5));
    }

    #[test]
    fn sub() {
        let result = Complex::new(5.0, 1.0) - Complex::new(2.0, 4.0);
        assert_eq!(result, Complex::new(3.0, -3.0));
    }

    #[test]
    fn sub_self_is_zero() {
        let z = Complex::new(1.5, -2.5);
        assert_eq!(z - z, Complex::zero());
    }

    // ---- scalar multiplication / division ----

    #[test]
    fn mul_by_scalar() {
        assert_eq!(Complex::new(1.0, -2.0) * 3.0, Complex::new(3.0, -6.0));
    }

    #[test]
    fn mul_by_zero_scalar() {
        assert_eq!(Complex::new(1.0, -2.0) * 0.0, Complex::zero());
    }

    #[test]
    fn div_by_scalar() {
        assert_eq!(Complex::new(3.0, -6.0) / 3.0, Complex::new(1.0, -2.0));
    }

    #[test]
    fn mul_then_div_by_scalar_round_trips() {
        let z = Complex::new(1.25, -0.75);
        assert_complex_close((z * 7.0) / 7.0, z);
    }

    #[test]
    fn div_by_zero_scalar_gives_infinities() {
        // Documents the current behaviour: IEEE 754 semantics, no panic.
        let result: Complex = Complex::new(1.0, -1.0) / 0.0;
        assert_eq!(result.real, f64::INFINITY);
        assert_eq!(result.imag, f64::NEG_INFINITY);
    }

    // ---- complex multiplication ----

    #[test]
    fn mul_complex() {
        // (1 + 2i)(3 + 4i) = 3 + 4i + 6i + 8i² = -5 + 10i
        let result = Complex::new(1.0, 2.0) * Complex::new(3.0, 4.0);
        assert_eq!(result, Complex::new(-5.0, 10.0));
    }

    #[test]
    fn i_squared_is_minus_one() {
        assert_eq!(I * I, Complex::new(-1.0, 0.0));
    }

    #[test]
    fn mul_by_zero_complex() {
        let z = Complex::new(1.5, -2.5);
        assert_eq!(z * Complex::zero(), Complex::zero());
    }

    #[test]
    fn mul_by_i_rotates_a_quarter_turn() {
        let z = Complex::new(2.0, 1.0);
        let rotated = z * I;
        assert_eq!(rotated, Complex::new(-1.0, 2.0));
        assert_close(rotated.phase(), z.phase() + PI / 2.0);
    }
}
