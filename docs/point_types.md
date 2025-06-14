# Course Point Types

The canonical set of course point types is defined in `Profile.xlsx` in the
[Garmin FIT SDK](https://developer.garmin.com/fit/download/), as the
`course_point` enum in the Types tab.  But the real question is how these
behave in practice on devices.

## On Garmin devices

Here's how different course points appear in "Up Ahead" on a Fenix 7 with
software version 21.19.  There are a few different cases to consider.  Types
listing an icon appear in Up Ahead with that icon, and also on the course map
with a similar but monochrome icon.  Types marked with (1) are missing
entirely from Up Ahead, but while they lack an icon on the map, the course
point's name will appear in the correct map location.  Finally, types marked
with (2) are missing from Up Ahead but appear on the map with an appropriate
icon.

| Type               | Fenix 7                               |
|--------------------|---------------------------------------|
| `generic`          | ![Generic](img/sample00a.png)         |
| `summit`           | ![Summit](img/sample00b.png)          |
| `valley`           | ![Valley](img/sample00c.png)          |
| `water`            | ![Water](img/sample00d.png)           |
| `food`             | ![Food](img/sample01a.png)            |
| `danger`           | ![Danger](img/sample01b.png)          |
| `left`             | (1)                                   |
| `right`            | (1)                                   |
| `straight`         | (1)                                   |
| `first_aid`        | ![FirstAid](img/sample02a.png)        |
| `fourth_category`  | ![FourthCategory](img/sample02b.png)  |
| `third_category`   | ![ThirdCategory](img/sample02c.png)   |
| `second_category`  | ![SecondCategory](img/sample03a.png)  |
| `first_category`   | ![FirstCategory](img/sample03b.png)   |
| `hors_category`    | ![HorsCategory](img/sample03c.png)    |
| `sprint`           | ![Sprint](img/sample03d.png)          |
| `left_fork`        | (1)                                   |
| `right_fork`       | (1)                                   |
| `middle_fork`      | (1)                                   |
| `slight_left`      | (1)                                   |
| `sharp_left`       | (1)                                   |
| `slight_right`     | (1)                                   |
| `sharp_right`      | (1)                                   |
| `u_turn`           | (1)                                   |
| `segment_start`    | (2)                                   |
| `segment_end`      | (2)                                   |
| `campsite`         | ![Campsite](img/sample06a.png)        |
| `aid_station`      | ![AidStation](img/sample06b.png)      |
| `rest_area`        | ![RestArea](img/sample07a.png)        |
| `general_distance` | ![GeneralDistance](img/sample07b.png) |
| `service`          | ![Service](img/sample07c.png)         |
| `energy_gel`       | ![EnergyGel](img/sample07d.png)       |
| `sports_drink`     | ![SportsDrink](img/sample08a.png)     |
| `mile_marker`      | ![MileMarker](img/sample08b.png)      |
| `checkpoint`       | ![Checkpoint](img/sample08c.png)      |
| `shelter`          | ![Shelter](img/sample08d.png)         |
| `meeting_spot`     | ![MeetingSpot](img/sample09a.png)     |
| `overlook`         | ![Overlook](img/sample09b.png)        |
| `toilet`           | ![Toilet](img/sample09c.png)          |
| `shower`           | (2)                                   |
| `gear`             | ![Gear](img/sample10a.png)            |
| `sharp_curve`      | ![SharpCurve](img/sample10b.png)      |
| `steep_incline`    | ![SteepIncline](img/sample10c.png)    |
| `tunnel`           | ![Tunnel](img/sample10d.png)          |
| `bridge`           | ![Bridge](img/sample11a.png)          |
| `obstacle`         | ![Obstacle](img/sample11b.png)        |
| `crossing`         | ![Crossing](img/sample11c.png)        |
| `store`            | ![Store](img/sample11d.png)           |
| `transition`       | ![Transition](img/sample12a.png)      |
| `navaid`           | ![Navaid](img/sample12b.png)          |
| `transport`        | ![Transport](img/sample12c.png)       |
| `alert`            | ![Alert](img/sample12d.png)           |
| `info`             | ![Info](img/sample13a.png)            |

## RideWithGPS POIs

| Type           | Icon                                            |
|----------------|-------------------------------------------------|
| Information    | ![Information](img/rwgps-information.png)       |
| Caution        | ![Caution](img/rwgps-caution.png)               |
| Hospital       | ![Hospital](img/rwgps-hospital.png)             |
| First Aid      | ![First Aid](img/rwgps-first-aid.png)           |
| Aid Station    | ![Aid Station](img/rwgps-aid-station.png)       |
| Restroom       | ![Restroom](img/rwgps-restroom.png)             |
| Shower         | ![Shower](img/rwgps-shower.png)                 |
| Water          | ![Water](img/rwgps-water.png)                   |
| Parking        | ![Parking](img/rwgps-parking.png)               |
| Gas Station    | ![Gas Station](img/rwgps-gas-station.png)       |
| Transit Center | ![Transit Center](img/rwgps-transit-center.png) |
| Ferry          | ![Ferry](img/rwgps-ferry.png)                   |
| Library        | ![Library](img/rwgps-library.png)               |
| Monument       | ![Monument](img/rwgps-monument.png)             |
| Viewpoint      | ![Viewpoint](img/rwgps-viewpoint.png)           |
| Trailhead      | ![Trailhead](img/rwgps-trailhead.png)           |
| Camping        | ![Camping](img/rwgps-camping.png)               |
| Park           | ![Park](img/rwgps-park.png)                     |
| Summit         | ![Summit](img/rwgps-summit.png)                 |
| Rest Stop      | ![Rest Stop](img/rwgps-rest-stop.png)           |
| Swimming       | ![Swimming](img/rwgps-swimming.png)             |
| Geocache       | ![Geocache](img/rwgps-geocache.png)             |
| Food           | ![Food](img/rwgps-food.png)                     |
| Bar            | ![Bar](img/rwgps-bar.png)                       |
| Coffee         | ![Coffee](img/rwgps-coffee.png)                 |
| Winery         | ![Winery](img/rwgps-winery.png)                 |
| Lodging        | ![Lodging](img/rwgps-lodging.png)               |
