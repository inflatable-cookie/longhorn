use std::{error::Error, fmt, marker::PhantomData};

use serde::{Deserialize, Serialize};

/// Marker for native physical-pixel geometry.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub enum PhysicalSpace {}

/// Marker for global screen-DIP geometry.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub enum ScreenSpace {}

/// A signed coordinate in a named integral coordinate space.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct Coordinate<Space> {
    value: i32,
    #[serde(skip)]
    space: PhantomData<Space>,
}

impl<Space> Coordinate<Space> {
    /// Constructs a coordinate.
    #[must_use]
    pub const fn new(value: i32) -> Self {
        Self {
            value,
            space: PhantomData,
        }
    }

    /// Returns the signed coordinate value.
    #[must_use]
    pub const fn get(self) -> i32 {
        self.value
    }
}

/// Signed physical-pixel coordinate.
pub type PhysicalPx = Coordinate<PhysicalSpace>;

/// Signed global screen-DIP coordinate.
pub type ScreenDip = Coordinate<ScreenSpace>;

/// A point in a named integral coordinate space.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(bound = "")]
pub struct Point<Space> {
    x: Coordinate<Space>,
    y: Coordinate<Space>,
}

impl<Space> Point<Space> {
    /// Constructs a point from signed coordinate values.
    #[must_use]
    pub const fn new(x: i32, y: i32) -> Self {
        Self {
            x: Coordinate::new(x),
            y: Coordinate::new(y),
        }
    }

    /// Constructs a point from typed coordinates.
    #[must_use]
    pub const fn from_coordinates(x: Coordinate<Space>, y: Coordinate<Space>) -> Self {
        Self { x, y }
    }

    /// Returns the horizontal coordinate.
    #[must_use]
    pub const fn x(&self) -> Coordinate<Space> {
        Coordinate::new(self.x.value)
    }

    /// Returns the vertical coordinate.
    #[must_use]
    pub const fn y(&self) -> Coordinate<Space> {
        Coordinate::new(self.y.value)
    }
}

/// A non-negative size in a named integral coordinate space.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(bound = "")]
pub struct Size<Space> {
    width: u32,
    height: u32,
    #[serde(skip)]
    space: PhantomData<Space>,
}

impl<Space> Size<Space> {
    /// Constructs a size.
    #[must_use]
    pub const fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            space: PhantomData,
        }
    }

    /// Returns the width.
    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }

    /// Returns the height.
    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }

    /// Returns whether either extent is zero.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.width == 0 || self.height == 0
    }
}

/// A translation in a named integral coordinate space.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(bound = "")]
pub struct Vector<Space> {
    dx: i32,
    dy: i32,
    #[serde(skip)]
    space: PhantomData<Space>,
}

impl<Space> Vector<Space> {
    /// Constructs a translation.
    #[must_use]
    pub const fn new(dx: i32, dy: i32) -> Self {
        Self {
            dx,
            dy,
            space: PhantomData,
        }
    }

    /// Returns the horizontal delta.
    #[must_use]
    pub const fn dx(&self) -> i32 {
        self.dx
    }

    /// Returns the vertical delta.
    #[must_use]
    pub const fn dy(&self) -> i32 {
        self.dy
    }
}

/// An axis-aligned rectangle in a named integral coordinate space.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(bound = "")]
pub struct Rect<Space> {
    origin: Point<Space>,
    size: Size<Space>,
}

impl<Space> Rect<Space> {
    /// Constructs a rectangle.
    #[must_use]
    pub const fn new(origin: Point<Space>, size: Size<Space>) -> Self {
        Self { origin, size }
    }

    /// Returns the rectangle origin.
    #[must_use]
    pub const fn origin(&self) -> Point<Space> {
        Point::new(self.origin.x.value, self.origin.y.value)
    }

    /// Returns the rectangle size.
    #[must_use]
    pub const fn size(&self) -> Size<Space> {
        Size::new(self.size.width, self.size.height)
    }

    /// Returns the exact rectangle area.
    #[must_use]
    pub const fn area(&self) -> u64 {
        self.size.width as u64 * self.size.height as u64
    }

    /// Returns whether this half-open rectangle contains a point.
    #[must_use]
    pub fn contains_point(&self, point: &Point<Space>) -> bool {
        let x = i64::from(point.x.value);
        let y = i64::from(point.y.value);
        let left = i64::from(self.origin.x.value);
        let top = i64::from(self.origin.y.value);

        x >= left && x < self.right() && y >= top && y < self.bottom()
    }

    /// Returns whether this rectangle completely contains another rectangle.
    #[must_use]
    pub fn contains_rect(&self, other: &Self) -> bool {
        let left = i64::from(self.origin.x.value);
        let top = i64::from(self.origin.y.value);
        let other_left = i64::from(other.origin.x.value);
        let other_top = i64::from(other.origin.y.value);

        other_left >= left
            && other_top >= top
            && other.right() <= self.right()
            && other.bottom() <= self.bottom()
    }

    /// Returns the positive-area intersection of two rectangles.
    #[must_use]
    pub fn intersection(&self, other: &Self) -> Option<Self> {
        let left = i64::from(self.origin.x.value).max(i64::from(other.origin.x.value));
        let top = i64::from(self.origin.y.value).max(i64::from(other.origin.y.value));
        let right = self.right().min(other.right());
        let bottom = self.bottom().min(other.bottom());

        if right <= left || bottom <= top {
            return None;
        }

        Some(Self::new(
            Point::new(i32::try_from(left).ok()?, i32::try_from(top).ok()?),
            Size::new(
                u32::try_from(right - left).ok()?,
                u32::try_from(bottom - top).ok()?,
            ),
        ))
    }

    /// Applies a checked translation.
    pub fn checked_translate(&self, vector: &Vector<Space>) -> Result<Self, GeometryError> {
        let x = self
            .origin
            .x
            .value
            .checked_add(vector.dx)
            .ok_or(GeometryError::CoordinateOverflow)?;
        let y = self
            .origin
            .y
            .value
            .checked_add(vector.dy)
            .ok_or(GeometryError::CoordinateOverflow)?;

        Ok(Self::new(Point::new(x, y), self.size()))
    }

    /// Resizes and moves the rectangle until it is fully inside `bounds`.
    ///
    /// The requested minimum is capped by the available bounds. A rectangle
    /// already contained with a sufficient size is returned unchanged.
    pub fn fit_within(
        &self,
        bounds: &Self,
        minimum_size: &Size<Space>,
    ) -> Result<Self, GeometryError> {
        require_nonempty_bounds(bounds)?;

        let width = self
            .size
            .width
            .max(minimum_size.width)
            .min(bounds.size.width);
        let height = self
            .size
            .height
            .max(minimum_size.height)
            .min(bounds.size.height);
        let rightmost_x = bounds.right() - i64::from(width);
        let bottommost_y = bounds.bottom() - i64::from(height);
        let x = i64::from(self.origin.x.value).clamp(i64::from(bounds.origin.x.value), rightmost_x);
        let y =
            i64::from(self.origin.y.value).clamp(i64::from(bounds.origin.y.value), bottommost_y);

        Ok(Self::new(
            Point::new(checked_i32(x)?, checked_i32(y)?),
            Size::new(width, height),
        ))
    }

    /// Moves the rectangle enough to expose an explicit minimum extent.
    ///
    /// Size is preserved. Each requested visible extent is capped by the
    /// rectangle and available bounds.
    pub fn ensure_minimum_visible(
        &self,
        bounds: &Self,
        minimum_visible: &Size<Space>,
    ) -> Result<Self, GeometryError> {
        require_nonempty_bounds(bounds)?;

        let visible_width = minimum_visible
            .width
            .min(self.size.width)
            .min(bounds.size.width);
        let visible_height = minimum_visible
            .height
            .min(self.size.height)
            .min(bounds.size.height);
        let minimum_x =
            i64::from(bounds.origin.x.value) - i64::from(self.size.width - visible_width);
        let maximum_x = bounds.right() - i64::from(visible_width);
        let minimum_y =
            i64::from(bounds.origin.y.value) - i64::from(self.size.height - visible_height);
        let maximum_y = bounds.bottom() - i64::from(visible_height);
        let x = i64::from(self.origin.x.value).clamp(minimum_x, maximum_x);
        let y = i64::from(self.origin.y.value).clamp(minimum_y, maximum_y);

        Ok(Self::new(
            Point::new(checked_i32(x)?, checked_i32(y)?),
            self.size(),
        ))
    }

    fn right(&self) -> i64 {
        i64::from(self.origin.x.value) + i64::from(self.size.width)
    }

    fn bottom(&self) -> i64 {
        i64::from(self.origin.y.value) + i64::from(self.size.height)
    }
}

fn require_nonempty_bounds<Space>(bounds: &Rect<Space>) -> Result<(), GeometryError> {
    if bounds.size.is_empty() {
        Err(GeometryError::EmptyBounds)
    } else {
        Ok(())
    }
}

fn checked_i32(value: i64) -> Result<i32, GeometryError> {
    i32::try_from(value).map_err(|_| GeometryError::CoordinateOverflow)
}

/// Checked integral geometry failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GeometryError {
    /// A coordinate result could not be represented as `i32`.
    CoordinateOverflow,
    /// A clamp operation received bounds with no positive area.
    EmptyBounds,
}

impl fmt::Display for GeometryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CoordinateOverflow => formatter.write_str("coordinate result overflowed i32"),
            Self::EmptyBounds => formatter.write_str("clamp bounds must have positive area"),
        }
    }
}

impl Error for GeometryError {}

/// Physical-pixel point.
///
/// Coordinate spaces cannot be substituted:
///
/// ```compile_fail
/// use longhorn_core::{PhysicalPoint, ScreenPoint};
///
/// let physical = PhysicalPoint::new(10, 20);
/// let _screen: ScreenPoint = physical;
/// ```
pub type PhysicalPoint = Point<PhysicalSpace>;

/// Physical-pixel size.
pub type PhysicalSize = Size<PhysicalSpace>;

/// Physical-pixel translation.
pub type PhysicalVector = Vector<PhysicalSpace>;

/// Physical-pixel rectangle.
pub type PhysicalRect = Rect<PhysicalSpace>;

/// Global screen-DIP point.
pub type ScreenPoint = Point<ScreenSpace>;

/// Global screen-DIP size.
pub type ScreenSize = Size<ScreenSpace>;

/// Global screen-DIP translation.
pub type ScreenVector = Vector<ScreenSpace>;

/// Global screen-DIP rectangle.
pub type ScreenRect = Rect<ScreenSpace>;
