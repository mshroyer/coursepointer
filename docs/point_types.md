# Course Point Types

The canonical set of course point types is defined in `Profile.xlsx` in the
[Garmin FIT SDK](https://developer.garmin.com/fit/download/), as the
`course_point` enum in the Types tab.  But the real question is how these
behave in practice on devices.

This document's purpose is to identify:

1. What subset of course point types is safe to use across devices.
2. How to map exported waypoints from popular apps like Ride with GPS and Gaia
   GPS to that subset.

## On Garmin devices

Here's how different course points appear in "Up Ahead" on a Fenix 7 with
software version 21.19.  There are a few different cases to consider.  Types
listing an icon appear in Up Ahead with that icon, and also on the course map
with a similar but monochrome icon.  Types marked with (1) are missing
entirely from Up Ahead, but while they lack an icon on the map, the course
point's name will appear in the correct map location.  Finally, types marked
with (2) are missing from Up Ahead but appear on the map with an appropriate
icon.

| Type               | Fenix 7                               | Connect |
|--------------------|---------------------------------------|---------|
| `generic`          | ![Generic](img/sample00a.png)         | yes     |
| `summit`           | ![Summit](img/sample00b.png)          | yes     |
| `valley`           | ![Valley](img/sample00c.png)          | yes     |
| `water`            | ![Water](img/sample00d.png)           | yes     |
| `food`             | ![Food](img/sample01a.png)            | yes     |
| `danger`           | ![Danger](img/sample01b.png)          | yes     |
| `left`             | (1)                                   | no      |
| `right`            | (1)                                   | no      |
| `straight`         | (1)                                   | no      |
| `first_aid`        | ![FirstAid](img/sample02a.png)        | yes     |
| `fourth_category`  | ![FourthCategory](img/sample02b.png)  | yes     |
| `third_category`   | ![ThirdCategory](img/sample02c.png)   | yes     |
| `second_category`  | ![SecondCategory](img/sample03a.png)  | yes     |
| `first_category`   | ![FirstCategory](img/sample03b.png)   | yes     |
| `hors_category`    | ![HorsCategory](img/sample03c.png)    | yes     |
| `sprint`           | ![Sprint](img/sample03d.png)          | yes     |
| `left_fork`        | (1)                                   | no      |
| `right_fork`       | (1)                                   | no      |
| `middle_fork`      | (1)                                   | no      |
| `slight_left`      | (1)                                   | no      |
| `sharp_left`       | (1)                                   | no      |
| `slight_right`     | (1)                                   | no      |
| `sharp_right`      | (1)                                   | no      |
| `u_turn`           | (1)                                   | no      |
| `segment_start`    | (2)                                   | no      |
| `segment_end`      | (2)                                   | no      |
| `campsite`         | ![Campsite](img/sample06a.png)        | yes     |
| `aid_station`      | ![AidStation](img/sample06b.png)      | yes     |
| `rest_area`        | ![RestArea](img/sample07a.png)        | yes     |
| `general_distance` | ![GeneralDistance](img/sample07b.png) | yes     |
| `service`          | ![Service](img/sample07c.png)         | yes     |
| `energy_gel`       | ![EnergyGel](img/sample07d.png)       | yes     |
| `sports_drink`     | ![SportsDrink](img/sample08a.png)     | yes     |
| `mile_marker`      | ![MileMarker](img/sample08b.png)      | yes     |
| `checkpoint`       | ![Checkpoint](img/sample08c.png)      | yes     |
| `shelter`          | ![Shelter](img/sample08d.png)         | yes     |
| `meeting_spot`     | ![MeetingSpot](img/sample09a.png)     | yes     |
| `overlook`         | ![Overlook](img/sample09b.png)        | yes     |
| `toilet`           | ![Toilet](img/sample09c.png)          | yes     |
| `shower`           | ![Shower](img/sample09d.png)          | yes     |
| `gear`             | ![Gear](img/sample10a.png)            | yes     |
| `sharp_curve`      | ![SharpCurve](img/sample10b.png)      | yes     |
| `steep_incline`    | ![SteepIncline](img/sample10c.png)    | yes     |
| `tunnel`           | ![Tunnel](img/sample10d.png)          | yes     |
| `bridge`           | ![Bridge](img/sample11a.png)          | yes     |
| `obstacle`         | ![Obstacle](img/sample11b.png)        | yes     |
| `crossing`         | ![Crossing](img/sample11c.png)        | yes     |
| `store`            | ![Store](img/sample11d.png)           | yes     |
| `transition`       | ![Transition](img/sample12a.png)      | yes     |
| `navaid`           | ![Navaid](img/sample12b.png)          | yes     |
| `transport`        | ![Transport](img/sample12c.png)       | yes     |
| `alert`            | ![Alert](img/sample12d.png)           | yes     |
| `info`             | ![Info](img/sample13a.png)            | yes     |

Bizarrely, the `shower` course point didn't show up at all my first time
testing this on my Fenix, but then rendered the next time, with the exact same
course file and firmware version.

The Connect column indicates whether the course point type appears when
imported into Garmin Connect, or can be created manually.  As of 2025-06-15,
it's possible to create additional "Obstacle Start" (type number 54) and
"Obstacle End" (type 55) which are absent from the current global
`Profile.xlsx`.

## RideWithGPS POIs

Ride with GPS has various POI types.  When these are exported as GPX
waypoints, they will correspond to certain `cmt` and `type` XML attributes.
Thanks to the new (if confusingly named) [Waypoints
feature](https://ridewithgps.com/news/11178-introducing-waypoints), when
exported in a FIT file, they also will correspond to certain FIT course point
types.

This table shows how the different POI types map to GPX and FIT types as of
2025-06-14.  The `cmt` XML attribute used in GPX exports corresponds to the
RWGPS POI type, while the `type` attribute always equals the FIT course point
type.

| Type              | Icon                                                  | GPX cmt attr        | GPX type attr | FIT type      |
|-------------------|-------------------------------------------------------|---------------------|---------------|---------------|
| Information       | ![Information](img/rwgps-information.png)             | `generic`           | `generic`     | `generic`     |
| Caution           | ![Caution](img/rwgps-caution.png)                     | `caution`           | `danger`      | `danger`      |
| Hospital          | ![Hospital](img/rwgps-hospital.png)                   | `hospital`          | `aid_station` | `aid_station` |
| First Aid         | ![First Aid](img/rwgps-first-aid.png)                 | `first_aid`         | `first_aid`   | `first_aid`   |
| Aid Station       | ![Aid Station](img/rwgps-aid-station.png)             | `aid_station`       | `aid_station` | `aid_station` |
| Restroom          | ![Restroom](img/rwgps-restroom.png)                   | `restroom`          | `toilet`      | `toilet`      |
| Shower            | ![Shower](img/rwgps-shower.png)                       | `shower`            | `shower`      | `shower`      |
| Water             | ![Water](img/rwgps-water.png)                         | `water`             | `water`       | `water`       |
| Parking           | ![Parking](img/rwgps-parking.png)                     | `parking`           | `service`     | `service`     |
| Gas Station       | ![Gas Station](img/rwgps-gas-station.png)             | `gas`               | `service`     | `service`     |
| Transit Center    | ![Transit Center](img/rwgps-transit-center.png)       | `transit`           | `transport`   | `transport`   |
| Ferry             | ![Ferry](img/rwgps-ferry.png)                         | `ferry`             | `transport`   | `transport`   |
| Library           | ![Library](img/rwgps-library.png)                     | `library`           | `info`        | `info`        |
| Monument          | ![Monument](img/rwgps-monument.png)                   | `monument`          | `info`        | `info`        |
| Viewpoint         | ![Viewpoint](img/rwgps-viewpoint.png)                 | `viewpoint`         | `overlook`    | `overlook`    |
| Trailhead         | ![Trailhead](img/rwgps-trailhead.png)                 | `trailhead`         | `info`        | `info`        |
| Camping           | ![Camping](img/rwgps-camping.png)                     | `camping`           | `campsite`    | `campsite`    |
| Park              | ![Park](img/rwgps-park.png)                           | `park`              | `rest_area`   | `rest_area`   |
| Summit            | ![Summit](img/rwgps-summit.png)                       | `summit`            | `summit`      | `summit`      |
| Rest Stop         | ![Rest Stop](img/rwgps-rest-stop.png)                 | `rest_stop`         | `rest_area`   | `rest_area`   |
| Swimming          | ![Swimming](img/rwgps-swimming.png)                   | `swimming`          | `rest_area`   | `rest_area`   |
| Geocache          | ![Geocache](img/rwgps-geocache.png)                   | `geocache`          | `info`        | `info`        |
| Food              | ![Food](img/rwgps-food.png)                           | `food`              | `food`        | `food`        |
| Bar               | ![Bar](img/rwgps-bar.png)                             | `bar`               | `food`        | `food`        |
| Coffee            | ![Coffee](img/rwgps-coffee.png)                       | `coffee`            | `food`        | `food`        |
| Winery            | ![Winery](img/rwgps-winery.png)                       | `winery`            | `food`        | `food`        |
| Lodging           | ![Lodging](img/rwgps-lodging.png)                     | `lodging`           | `shelter`     | `shelter`     |
| Convenience Store | ![Convenience Store](img/rwgps-convenience-store.png) | `convenience_store` | `store`       | `store`       |
| Shopping          | ![Shopping](img/rwgps-shopping.png)                   | `shopping`          | `store`       | `store`       |
| ATM               | ![ATM](img/rwgps-atm.png)                             | `atm`               | `service`     | `service`     |
| Bike Shop         | ![Bike Shop](img/rwgps-bike-shop.png)                 | `bike_shop`         | `gear`        | `gear`        |
| Bike Parking      | ![Bike Parking](img/rwgps-bike-parking.png)           | `bike_parking`      | `service`     | `service`     |
| Bike Share        | ![Bike Share](img/rwgps-bike-share.png)               | `bikeshare`         | `service`     | `service`     |
| Start             | ![Start](img/rwgps-start.png)                         | `start`             | `generic`     | `generic`     |
| Stop              | ![Stop](img/rwgps-stop.png)                           | `stop`              | `generic`     | `generic`     |
| Finish            | ![Finish](img/rwgps-finish.png)                       | `finish`            | `generic`     | `generic`     |
| Segment Start     | ![Segment Start](img/rwgps-segment-start.png)         | `segment_start`     | `generic`     | `generic`     |
| Segment End       | ![Segment End](img/rwgps-segment-end.png)             | `segment_end`       | `generic`     | `generic`     |
| Control           | ![Control](img/rwgps-control.png)                     | `control`           | `checkpoint`  | `checkpoint`  |

The full set of course point types used by Ride with GPS in FIT exports of
custom POIs and Waypoints (setting aside cues) is then:

- `generic`
- `summit`
- `water`
- `food`
- `danger`
- `first_aid`
- `campsite`
- `aid_station`
- `rest_area`
- `service`
- `checkpoint`
- `shelter`
- `overlook`
- `toilet`
- `shower`
- `gear`
- `store`
- `transport`
- `info`

This might represent a safe, conservative set of course point types to use in
FIT exports, as likely Ride with GPS has tested this more thoroughly than I
have.
