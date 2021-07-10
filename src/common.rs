pub type Result<T> = std::result::Result<T, anyhow::Error>;

pub trait PointConversion<To> {
    fn convert(&self) -> To;
}

impl PointConversion<geo::Point<f64>> for geo::Coordinate<i64> {
    fn convert(&self) -> geo::Point<f64> {
        geo::Point::new(self.x as f64, self.y as f64)
    }
}

impl PointConversion<geo::Coordinate<i64>> for geo::Point<f64> {
    fn convert(&self) -> geo::Coordinate<i64> {
        geo::Coordinate {
            x: self.x().trunc() as i64,
            y: self.y().trunc() as i64,
        }
    }
}
