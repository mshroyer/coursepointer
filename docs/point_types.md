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

| Type            | Fenix 7                                    |
|-----------------|--------------------------------------------|
| Generic         | ![Generic icon](img/sample00a.png)         |
| Summit          | ![Summit icon](img/sample00b.png)          |
| Valley          | ![Valley icon](img/sample00c.png)          |
| Water           | ![Water icon](img/sample00d.png)           |
| Food            | ![Food icon](img/sample01a.png)            |
| Danger          | ![Danger icon](img/sample01b.png)          |
| Left            | (1)                                        |
| Right           | (1)                                        |
| Straight        | (1)                                        |
| FirstAid        | ![FirstAid icon](img/sample02a.png)        |
| FourthCategory  | ![FourthCategory icon](img/sample02b.png)  |
| ThirdCategory   | ![ThirdCategory icon](img/sample02c.png)   |
| SecondCategory  | ![SecondCategory icon](img/sample03a.png)  |
| FirstCategory   | ![FirstCategory icon](img/sample03b.png)   |
| HorsCategory    | ![HorsCategory icon](img/sample03c.png)    |
| Sprint          | ![Sprint icon](img/sample03d.png)          |
| LeftFork        | (1)                                        |
| RightFork       | (1)                                        |
| MiddleFork      | (1)                                        |
| SlightLeft      | (1)                                        |
| SharpLeft       | (1)                                        |
| SlightRight     | (1)                                        |
| SharpRight      | (1)                                        |
| UTurn           | (1)                                        |
| SegmentStart    | (2)                                        |
| SegmentEnd      | (2)                                        |
| Campsite        | ![Campsite icon](img/sample06a.png)        |
| AidStation      | ![AidStation icon](img/sample06b.png)      |
| RestArea        | ![RestArea icon](img/sample07a.png)        |
| GeneralDistance | ![GeneralDistance icon](img/sample07b.png) |
| Service         | ![Service icon](img/sample07c.png)         |
| EnergyGel       | ![EnergyGel icon](img/sample07d.png)       |
| SportsDrink     | ![SportsDrink icon](img/sample08a.png)     |
| MileMarker      | ![MileMarker icon](img/sample08b.png)      |
| Checkpoint      | ![Checkpoint icon](img/sample08c.png)      |
| Shelter         | ![Shelter icon](img/sample08d.png)         |
| MeetingSpot     | ![MeetingSpot icon](img/sample09a.png)     |
| Overlook        | ![Overlook icon](img/sample09b.png)        |
| Toilet          | ![Toilet icon](img/sample09c.png)          |
| Shower          | (2)                                        |
| Gear            | ![Gear icon](img/sample10a.png)            |
| SharpCurve      | ![SharpCurve icon](img/sample10b.png)      |
| SteepIncline    | ![SteepIncline icon](img/sample10c.png)    |
| Tunnel          | ![Tunnel icon](img/sample10d.png)          |
| Bridge          | ![Bridge icon](img/sample11a.png)          |
| Obstacle        | ![Obstacle icon](img/sample11b.png)        |
| Crossing        | ![Crossing icon](img/sample11c.png)        |
| Store           | ![Store icon](img/sample11d.png)           |
| Transition      | ![Transition icon](img/sample12a.png)      |
| Navaid          | ![Navaid icon](img/sample12b.png)          |
| Transport       | ![Transport icon](img/sample12c.png)       |
| Alert           | ![Alert icon](img/sample12d.png)           |
| Info            | ![Info icon](img/sample13a.png)            |

