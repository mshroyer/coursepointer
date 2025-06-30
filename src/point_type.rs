use std::str::FromStr;

use phf::phf_map;
use tracing::warn;

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
        GpxCreator::GaiaGps => get_gaiagps_point_type(waypoint),
        GpxCreator::RideWithGps => get_ridewithgps_point_type(waypoint),
        GpxCreator::Unknown => CoursePointType::Generic,
    }
}

static GAIAGPS_SYMS: phf::Map<&'static str, CoursePointType> = phf_map! {
    "airport-24" => CoursePointType::Transport,
    "bear" => CoursePointType::Danger,
    "bicycle-24" => CoursePointType::Transport,
    "binoculars" => CoursePointType::Overlook,
    "water" => CoursePointType::Water,
    "bridge" => CoursePointType::Bridge,
    "building-24" => CoursePointType::Shelter,
    "bus" => CoursePointType::Transport,
    "cafe-24" => CoursePointType::Food,
    "cairn" => CoursePointType::Navaid,
    "camera-24" => CoursePointType::Overlook,
    "campsite-24" => CoursePointType::Campsite,
    "car-24" => CoursePointType::Transport,
    "cemetery-24" => CoursePointType::Overlook,
    "danger-24" => CoursePointType::Danger,
    "dog-park-24" => CoursePointType::RestArea,
    "electric" => CoursePointType::Service,
    "emergency-telephone-24" => CoursePointType::Service,
    "fast-food-24" => CoursePointType::Food,
    "fence" => CoursePointType::Obstacle,
    "fire-lookout" => CoursePointType::Overlook,
    "fire-station-24" => CoursePointType::FirstAid,
    "fish" => CoursePointType::Food,
    "fuel-24" => CoursePointType::Service,
    "gate" => CoursePointType::Obstacle,
    "geyser" => CoursePointType::Overlook,
    "ghost-town" => CoursePointType::Overlook,
    "ground-blind" => CoursePointType::Shelter,
    "helipad" => CoursePointType::Transport,
    "heliport-24" => CoursePointType::Transport,
    "hospital-24" => CoursePointType::AidStation,
    "information" => CoursePointType::Info,
    "known-route" => CoursePointType::Transition,
    "lighthouse-24" => CoursePointType::Overlook,
    "lodging-24" => CoursePointType::Shelter,
    "market" => CoursePointType::Store,
    "minefield-24" => CoursePointType::Danger,
    "moose" => CoursePointType::Danger,
    "mud" => CoursePointType::Obstacle,
    "museum" => CoursePointType::Info,
    "no-admittance-1" => CoursePointType::Obstacle,
    "no-admittance-2" => CoursePointType::Obstacle,
    "number-1" => CoursePointType::FirstCategory,
    "number-2" => CoursePointType::SecondCategory,
    "number-3" => CoursePointType::ThirdCategory,
    "number-4" => CoursePointType::FourthCategory,
    "park-24" => CoursePointType::RestArea,
    "parking-24" => CoursePointType::Transport,
    "peak" => CoursePointType::Summit,
    "petroglyph" => CoursePointType::Overlook,
    "picnic" => CoursePointType::RestArea,
    "playground-24" => CoursePointType::RestArea,
    "police" => CoursePointType::Service,
    "potable-water" => CoursePointType::Water,
    "rail-24" => CoursePointType::Transport,
    "railroad" => CoursePointType::Crossing,
    "ranger-station" => CoursePointType::Shelter,
    "restaurant-24" => CoursePointType::Food,
    "resupply" => CoursePointType::Store,
    "ruins" => CoursePointType::Overlook,
    "rv-park" => CoursePointType::RestArea,
    "saddle" => CoursePointType::Gear,
    "shelter" => CoursePointType::Shelter,
    "shower" => CoursePointType::Shower,
    "snowmobile" => CoursePointType::Transport,
    "steps" => CoursePointType::SteepIncline,
    "toilets-24" => CoursePointType::Toilet,
    "trail-camera" => CoursePointType::Overlook,
    "trailhead" => CoursePointType::Navaid,
    "tree-fall" => CoursePointType::Obstacle,
    "tree-stand" => CoursePointType::Overlook,
    "volcano" => CoursePointType::Overlook,
    "water-24" => CoursePointType::Water,
    "waterfall" => CoursePointType::Overlook,
    "wood" => CoursePointType::Service,
};

fn get_gaiagps_point_type(waypoint: &GpxWaypoint) -> CoursePointType {
    match &waypoint.sym {
        Some(t) => match GAIAGPS_SYMS.get(t) {
            Some(p) => *p,
            None => CoursePointType::Generic,
        },
        None => {
            warn!("Gaia GPS GPX missing waypoint sym");
            CoursePointType::Generic
        }
    }
}

fn get_ridewithgps_point_type(waypoint: &GpxWaypoint) -> CoursePointType {
    match &waypoint.type_ {
        Some(t) => CoursePointType::from_str(t).unwrap_or(CoursePointType::Generic),
        None => {
            warn!("Ride with GPS GPX missing waypoint type");
            CoursePointType::Generic
        }
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use crate::fit::CoursePointType;
    use crate::geo_point;
    use crate::gpx::GpxWaypoint;
    use crate::point_type::{
        GpxCreator, get_gaiagps_point_type, get_gpx_creator, get_ridewithgps_point_type,
    };

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
                point: geo_point!(0.0, 0.0)?,
            }),
            CoursePointType::Generic
        );

        assert_eq!(
            get_ridewithgps_point_type(&GpxWaypoint {
                name: "Foo".to_string(),
                cmt: None,
                sym: None,
                type_: Some("generic".to_string()),
                point: geo_point!(0.0, 0.0)?,
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
                point: geo_point!(0.0, 0.0)?,
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
                point: geo_point!(0.0, 0.0)?,
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
                point: geo_point!(0.0, 0.0)?,
            }),
            CoursePointType::Generic
        );

        Ok(())
    }

    #[test]
    fn gaiagps_point_type() -> Result<()> {
        assert_eq!(
            get_gaiagps_point_type(&GpxWaypoint {
                name: "Foo".to_string(),
                cmt: None,
                sym: None,
                type_: None,
                point: geo_point!(0.0, 0.0)?,
            }),
            CoursePointType::Generic
        );

        // Unmapped string
        assert_eq!(
            get_gaiagps_point_type(&GpxWaypoint {
                name: "Foo".to_string(),
                cmt: None,
                sym: Some("UFO".to_string()),
                type_: None,
                point: geo_point!(0.0, 0.0)?,
            }),
            CoursePointType::Generic
        );

        // Mapped string
        assert_eq!(
            get_gaiagps_point_type(&GpxWaypoint {
                name: "Foo".to_string(),
                cmt: None,
                sym: Some("bus".to_string()),
                type_: None,
                point: geo_point!(0.0, 0.0)?,
            }),
            CoursePointType::Transport
        );

        Ok(())
    }
}
