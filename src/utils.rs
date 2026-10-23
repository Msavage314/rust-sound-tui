#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Complex {
    pub real: f64,
    pub imag: f64,
}
impl Complex {
    pub const fn zero() -> Self {
        return Self {
            real: 0.0,
            imag: 0.0,
        };
    }
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
    pub fn norm(self) -> f64 {
        self.real.hypot(self.imag)
    }
}
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
impl core::ops::Div<f64> for Complex {
    type Output = Complex;
    fn div(self, rhs: f64) -> Self::Output {
        return Self {
            real: self.real / rhs,
            imag: self.imag / rhs,
        };
    }
}
impl core::ops::Mul<f64> for Complex {
    type Output = Complex;
    fn mul(self, rhs: f64) -> Self::Output {
        return Self {
            real: self.real * rhs,
            imag: self.imag * rhs,
        };
    }
}
impl core::ops::Mul<Self> for Complex {
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
impl From<f64> for Complex {
    fn from(real: f64) -> Self {
        Self { real, imag: 0.0 }
    }
}
