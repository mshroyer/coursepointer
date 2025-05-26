//! Abstract course elements

use coretypes::GeoPoint;
use coretypes::measure::Meters;
use geographic::{GeographicError, solve_inverse};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CourseError {
    #[error(transparent)]
    GeographicalError(#[from] GeographicError),
}

type Result<T> = std::result::Result<T, CourseError>;

pub struct CourseSet {
    courses: Vec<CourseBuilder>,
}

impl CourseSet {
    pub fn new() -> Self {
        Self {
            courses: Vec::new(),
        }
    }
}

const DEFAULT_COURSE_NAME : &str = "Untitled course";

pub struct CourseBuilder {
    records: Vec<Record>,
    name: Option<String>,
}

impl CourseBuilder {
    pub fn new() -> Self {
        Self {
            records: Vec::new(),
            name: None,
        }
    }
    
    pub fn set_name(&mut self, name: String) {
        self.name = Some(name);
    }

    pub fn add_record(&mut self, point: GeoPoint) -> Result<()> {
        match self.records.iter().last() {
            Some(last) => {
                // TODO: Investigate using elevation-corrected distances
                let distance_increment = solve_inverse(&last.point, &point)?.geo_distance;
                self.records.push(Record {
                    point,
                    distance: last.distance + distance_increment,
                })
            }

            None => self.records.push(Record {
                point,
                distance: Meters(0.0),
            }),
        }
        Ok(())
    }
    
    pub fn records_len(&self) -> usize {
        self.records.len()
    }
    
    pub fn iter_records(&self) -> impl Iterator<Item = &Record> {
        self.records.iter()
    }
    
    pub fn total_distance(&self) -> Meters<f64> {
        self.records.iter().last().map(|x| x.distance).unwrap_or(Meters(0.0))
    }
    
    pub fn get_name(&self) -> &str {
        match &self.name {
            Some(name) => name.as_ref(),
            None => DEFAULT_COURSE_NAME,
        }
    }
}

pub struct Record {
    pub point: GeoPoint,
    pub distance: Meters<f64>,
}
