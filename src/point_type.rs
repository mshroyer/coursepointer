use std::str::FromStr;

use crate::fit::CoursePointType;
use crate::gpx::GpxWaypoint;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum GpxCreator {
    Unknown,
    GaiaGps,
    RideWithGps,
}

pub fn get_gpx_creator(creator: &str) -> GpxCreator {
    match creator {
        "GaiaGPS" => GpxCreator::GaiaGps,
        "http://ridewithgps.com/" => GpxCreator::RideWithGps,
        _ => GpxCreator::Unknown,
    }
}

pub fn get_course_point_type(creator: GpxCreator, waypoint: &GpxWaypoint) -> CoursePointType {
    match creator {
        GpxCreator::GaiaGps => get_gaiagps_point_type(&waypoint),
        GpxCreator::RideWithGps => get_ridewithgps_point_type(&waypoint),
        GpxCreator::Unknown => CoursePointType::Generic,
    }
}

fn get_gaiagps_point_type(waypoint: &GpxWaypoint) -> CoursePointType {
    CoursePointType::Generic
}

fn get_ridewithgps_point_type(waypoint: &GpxWaypoint) -> CoursePointType {
    match &waypoint.type_ {
        Some(t) => CoursePointType::from_str(t).unwrap_or_else(|e| CoursePointType::Generic),
        None => CoursePointType::Generic,
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use crate::fit::CoursePointType;
    use crate::geo_point;
    use crate::gpx::GpxWaypoint;
    use crate::point_type::{GpxCreator, get_gpx_creator, get_ridewithgps_point_type};

    #[test]
    fn gpx_creator() {
        assert_eq!(get_gpx_creator("GaiaGPS"), GpxCreator::GaiaGps);
        assert_eq!(
            get_gpx_creator("http://ridewithgps.com/"),
            GpxCreator::RideWithGps
        );
        assert_eq!(get_gpx_creator("AwesomeApp"), GpxCreator::Unknown);
    }

    #[test]
    fn ridewithgps_point_type() -> Result<()> {
        assert_eq!(
            get_ridewithgps_point_type(&GpxWaypoint {
                name: "Foo".to_string(),
                cmt: None,
                sym: None,
                type_: None,
                point: geo_point!(0.0, 0.0),
            }),
            CoursePointType::Generic
        );

        assert_eq!(
            get_ridewithgps_point_type(&GpxWaypoint {
                name: "Foo".to_string(),
                cmt: None,
                sym: None,
                type_: Some("generic".to_string()),
                point: geo_point!(0.0, 0.0),
            }),
            CoursePointType::Generic
        );

        // Should parse point types.
        assert_eq!(
            get_ridewithgps_point_type(&GpxWaypoint {
                name: "Foo".to_string(),
                cmt: None,
                sym: None,
                type_: Some("food".to_string()),
                point: geo_point!(0.0, 0.0),
            }),
            CoursePointType::Food
        );

        // Should parse in snake_case.
        assert_eq!(
            get_ridewithgps_point_type(&GpxWaypoint {
                name: "Foo".to_string(),
                cmt: None,
                sym: None,
                type_: Some("general_distance".to_string()),
                point: geo_point!(0.0, 0.0),
            }),
            CoursePointType::GeneralDistance
        );

        // Should fail to generic if the type cannot be parsed.
        assert_eq!(
            get_ridewithgps_point_type(&GpxWaypoint {
                name: "Foo".to_string(),
                cmt: None,
                sym: None,
                type_: Some("Wakka wakka".to_string()),
                point: geo_point!(0.0, 0.0),
            }),
            CoursePointType::Generic
        );

        Ok(())
    }
}
