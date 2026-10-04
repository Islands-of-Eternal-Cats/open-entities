//! Conversion between map units (decimals, as the outside world writes them) and the simulation's
//! integer milli-units.
//!
//! This is a boundary module: the only place in the simulation's data path that touches floats.
//! Everything the host writes — YAML, `spawn_entity` overrides, the JSON export, the JS API — keeps
//! map units as decimals. Inside, positions, targets, radii and ranges are `i32` milli-units, and
//! speeds and velocities are `i32` milli-units **per tick**.
//!
//! - In: `(v * 1000.0).round()` in `f64`, half away from zero. A value outside `i32` is an error.
//! - Out: `milli as f64 / 1000.0`, which is exact: the shortest form serde prints is the original
//!   decimal (up to three places).
//! - Speeds in: `round(units_per_second × TICK_MS)`; out: `per_tick / TICK_MS`. At `TICK_MS` = 50
//!   a speed is quantised to 0.02 units/s, and the export shows the speed actually simulated.

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::components::{BaseMoveSpeed, MoveTarget, Position, Velocity};
use crate::simulation::TICK_MS;

/// Milli-units per map unit.
pub const MILLI_PER_UNIT: i32 = 1000;

/// A map-unit value that has no `i32` milli-unit form: too large, or not a number.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OutOfRange(pub f64);

impl std::fmt::Display for OutOfRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "value {} is out of range for the simulation's i32 milli-units",
            self.0
        )
    }
}

impl std::error::Error for OutOfRange {}

fn round_to_i32(value: f64, original: f64) -> Result<i32, OutOfRange> {
    let rounded = value.round();
    if rounded.is_nan() || rounded < f64::from(i32::MIN) || rounded > f64::from(i32::MAX) {
        return Err(OutOfRange(original));
    }
    #[allow(clippy::cast_possible_truncation)] // range checked above
    Ok(rounded as i32)
}

/// Map units to milli-units.
///
/// # Errors
///
/// [`OutOfRange`] when the result does not fit in `i32` or `units` is not a number.
pub fn to_milli(units: f64) -> Result<i32, OutOfRange> {
    round_to_i32(units * f64::from(MILLI_PER_UNIT), units)
}

/// Milli-units to map units. Exact.
#[must_use]
pub fn from_milli(milli: i32) -> f64 {
    f64::from(milli) / f64::from(MILLI_PER_UNIT)
}

/// Units per second to milli-units per tick.
///
/// # Errors
///
/// [`OutOfRange`] when the result does not fit in `i32` or `units_per_second` is not a number.
pub fn speed_to_per_tick(units_per_second: f64) -> Result<i32, OutOfRange> {
    round_to_i32(units_per_second * f64::from(TICK_MS), units_per_second)
}

/// Milli-units per tick to units per second.
#[must_use]
pub fn speed_from_per_tick(per_tick: i32) -> f64 {
    f64::from(per_tick) / f64::from(TICK_MS)
}

impl Position {
    /// A position from map units.
    ///
    /// # Panics
    ///
    /// When a coordinate is out of `i32` milli-unit range; see [`to_milli`] for a checked form.
    #[must_use]
    pub fn from_units(x: f64, y: f64) -> Self {
        Self {
            x: to_milli(x).expect("x in range"),
            y: to_milli(y).expect("y in range"),
        }
    }
}

impl MoveTarget {
    /// A move target from map units.
    ///
    /// # Panics
    ///
    /// When a coordinate is out of `i32` milli-unit range; see [`to_milli`] for a checked form.
    #[must_use]
    pub fn from_units(x: f64, y: f64) -> Self {
        Self {
            x: to_milli(x).expect("x in range"),
            y: to_milli(y).expect("y in range"),
        }
    }
}

/// Wire form of [`Position`] and [`MoveTarget`]: map units.
#[derive(Serialize, Deserialize)]
struct PointRepr {
    x: f64,
    y: f64,
}

/// Wire form of [`Velocity`]: map units per second.
#[derive(Serialize, Deserialize)]
struct VelocityRepr {
    vx: f64,
    vy: f64,
}

fn point_in<'de, D: Deserializer<'de>>(deserializer: D) -> Result<(i32, i32), D::Error> {
    let repr = PointRepr::deserialize(deserializer)?;
    Ok((
        to_milli(repr.x).map_err(D::Error::custom)?,
        to_milli(repr.y).map_err(D::Error::custom)?,
    ))
}

fn point_out<S: Serializer>(x: i32, y: i32, serializer: S) -> Result<S::Ok, S::Error> {
    PointRepr {
        x: from_milli(x),
        y: from_milli(y),
    }
    .serialize(serializer)
}

impl Serialize for Position {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        point_out(self.x, self.y, serializer)
    }
}

impl<'de> Deserialize<'de> for Position {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let (x, y) = point_in(deserializer)?;
        Ok(Self { x, y })
    }
}

impl Serialize for MoveTarget {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        point_out(self.x, self.y, serializer)
    }
}

impl<'de> Deserialize<'de> for MoveTarget {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let (x, y) = point_in(deserializer)?;
        Ok(Self { x, y })
    }
}

impl Serialize for Velocity {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        VelocityRepr {
            vx: speed_from_per_tick(self.vx),
            vy: speed_from_per_tick(self.vy),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Velocity {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let repr = VelocityRepr::deserialize(deserializer)?;
        Ok(Self {
            vx: speed_to_per_tick(repr.vx).map_err(D::Error::custom)?,
            vy: speed_to_per_tick(repr.vy).map_err(D::Error::custom)?,
        })
    }
}

impl Serialize for BaseMoveSpeed {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        speed_from_per_tick(self.0).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for BaseMoveSpeed {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let units_per_second = f64::deserialize(deserializer)?;
        speed_to_per_tick(units_per_second)
            .map(Self)
            .map_err(D::Error::custom)
    }
}

/// Serde form of a bare distance — a mission radius, say: map units outside, milli-units inside.
///
/// For `#[serde(with = "crate::units::distance")]` on an `i32` milli-unit field.
pub mod distance {
    use serde::de::Error as _;

    use super::{Deserialize, Deserializer, Serialize, Serializer, from_milli, to_milli};

    /// Writes milli-units as map units.
    ///
    /// # Errors
    ///
    /// Whatever the serializer reports.
    #[allow(clippy::trivially_copy_pass_by_ref)] // the signature serde's `with` expects
    pub fn serialize<S: Serializer>(milli: &i32, serializer: S) -> Result<S::Ok, S::Error> {
        from_milli(*milli).serialize(serializer)
    }

    /// Reads map units as milli-units.
    ///
    /// # Errors
    ///
    /// When the value is not a number or out of `i32` milli-unit range.
    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<i32, D::Error> {
        to_milli(f64::deserialize(deserializer)?).map_err(D::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_milli_rounds_half_away_from_zero() {
        assert_eq!(to_milli(0.0005), Ok(1));
        assert_eq!(to_milli(-0.0005), Ok(-1));
        assert_eq!(to_milli(1.2344), Ok(1234));
    }

    #[test]
    fn to_milli_rejects_out_of_range_and_nan() {
        assert!(to_milli(2_147_483.647).is_ok());
        assert!(to_milli(2_147_483.648).is_err());
        assert!(to_milli(-2_147_483.649).is_err());
        assert!(to_milli(f64::NAN).is_err());
    }

    #[test]
    fn from_milli_prints_the_original_decimal() {
        for text in ["0.1", "-0.001", "12.345", "2147483.647", "-2147483.648"] {
            let milli = to_milli(text.parse().expect("number")).expect("in range");
            assert_eq!(from_milli(milli).to_string(), text);
        }
    }

    #[test]
    fn speed_conversion_uses_tick_length() {
        assert_eq!(speed_to_per_tick(0.5), Ok(25));
        assert_eq!(speed_to_per_tick(0.51), Ok(26));
        assert_eq!(speed_from_per_tick(26).to_string(), "0.52");
    }

    #[test]
    fn from_units_constructors() {
        assert_eq!(
            Position::from_units(1.5, -2.0),
            Position { x: 1500, y: -2000 }
        );
        assert_eq!(
            MoveTarget::from_units(0.001, 0.0),
            MoveTarget { x: 1, y: 0 }
        );
    }
}
