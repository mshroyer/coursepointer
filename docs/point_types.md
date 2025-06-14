# Course Point Types

The canonical set of course point types is defined in `Profile.xlsx` in the
[Garmin FIT SDK](https://developer.garmin.com/fit/download/), as the
`course_point` enum in the Types tab.  But the real question is how these
behave in practice on devices.

Here's how different course points appear in "Up Ahead" on a Fenix 7 with
software version 21.19.  There are a few different cases to consider.  Types
listing an icon appear in Up Ahead with that icon, and also on the course map
with a similar but monochrome icon.  Types marked with (1) are missing
entirely from Up Ahead, but while they lack an icon on the map, the course
point's name will appear in the correct map location.  Finally, types marked
with (2) are missing from Up Ahead but appear on the map with an appropriate
icon.

| Type               | Fenix 7                                    |
|--------------------|--------------------------------------------|
| `generic`          | ![Generic icon](img/sample00a.png)         |
| `summit`           | ![Summit icon](img/sample00b.png)          |
| `valley`           | ![Valley icon](img/sample00c.png)          |
| `water`            | ![Water icon](img/sample00d.png)           |
| `food`             | ![Food icon](img/sample01a.png)            |
| `danger`           | ![Danger icon](img/sample01b.png)          |
| `left`             | (1)                                        |
| `right`            | (1)                                        |
| `straight`         | (1)                                        |
| `first_aid`        | ![FirstAid icon](img/sample02a.png)        |
| `fourth_category`  | ![FourthCategory icon](img/sample02b.png)  |
| `third_category`   | ![ThirdCategory icon](img/sample02c.png)   |
| `second_category`  | ![SecondCategory icon](img/sample03a.png)  |
| `first_category`   | ![FirstCategory icon](img/sample03b.png)   |
| `hors_category`    | ![HorsCategory icon](img/sample03c.png)    |
| `sprint`           | ![Sprint icon](img/sample03d.png)          |
| `left_fork`        | (1)                                        |
| `right_fork`       | (1)                                        |
| `middle_fork`      | (1)                                        |
| `slight_left`      | (1)                                        |
| `sharp_left`       | (1)                                        |
| `slight_right`     | (1)                                        |
| `sharp_right`      | (1)                                        |
| `u_turn`           | (1)                                        |
| `segment_start`    | (2)                                        |
| `segment_end`      | (2)                                        |
| `campsite`         | ![Campsite icon](img/sample06a.png)        |
| `aid_station`      | ![AidStation icon](img/sample06b.png)      |
| `rest_area`        | ![RestArea icon](img/sample07a.png)        |
| `general_distance` | ![GeneralDistance icon](img/sample07b.png) |
| `service`          | ![Service icon](img/sample07c.png)         |
| `energy_gel`       | ![EnergyGel icon](img/sample07d.png)       |
| `sports_drink`     | ![SportsDrink icon](img/sample08a.png)     |
| `mile_marker`      | ![MileMarker icon](img/sample08b.png)      |
| `checkpoint`       | ![Checkpoint icon](img/sample08c.png)      |
| `shelter`          | ![Shelter icon](img/sample08d.png)         |
| `meeting_spot`     | ![MeetingSpot icon](img/sample09a.png)     |
| `overlook`         | ![Overlook icon](img/sample09b.png)        |
| `toilet`           | ![Toilet icon](img/sample09c.png)          |
| `shower`           | (2)                                        |
| `gear`             | ![Gear icon](img/sample10a.png)            |
| `sharp_curve`      | ![SharpCurve icon](img/sample10b.png)      |
| `steep_incline`    | ![SteepIncline icon](img/sample10c.png)    |
| `tunnel`           | ![Tunnel icon](img/sample10d.png)          |
| `bridge`           | ![Bridge icon](img/sample11a.png)          |
| `obstacle`         | ![Obstacle icon](img/sample11b.png)        |
| `crossing`         | ![Crossing icon](img/sample11c.png)        |
| `store`            | ![Store icon](img/sample11d.png)           |
| `transition`       | ![Transition icon](img/sample12a.png)      |
| `navaid`           | ![Navaid icon](img/sample12b.png)          |
| `transport`        | ![Transport icon](img/sample12c.png)       |
| `alert`            | ![Alert icon](img/sample12d.png)           |
| `info`             | ![Info icon](img/sample13a.png)            |

