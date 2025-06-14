# Course Point Types

The canonical set of course point types is defined in `Profile.xlsx` in the
[Garmin FIT SDK](https://developer.garmin.com/fit/download/), as the
`course_point` type.  But that just gives us a list of possible types to use.
The real question is, how do these point types behave when loaded onto Garmin
devices we care about?

Here are how different course point types appear in "Up Ahead" on a Fenix 7
running software version 21.19.  There are a few different cases to consider.
Types with an icon listed appear in Up Ahead with the designated icon, and
also on the course map with a similar but monochrome icon.  Types marked with
(1) are missing from Up Ahead, but while they also lack an icon on the map,
the course point's name will appear in the appropriate location on the course
map.  Finally, types marked with (2) are missing from Up Ahead but appear on
the map with an appropriate icon.

| Type            | Fenix 7                               |
|-----------------|---------------------------------------|
| Generic         | ![Generic](img/sample00a.png)         |
| Summit          | ![Summit](img/sample00b.png)          |
| Valley          | ![Valley](img/sample00c.png)          |
| Water           | ![Water](img/sample00d.png)           |
| Food            | ![Food](img/sample01a.png)            |
| Danger          | ![Danger](img/sample01b.png)          |
| Left            | (1)                                   |
| Right           | (1)                                   |
| Straight        | (1)                                   |
| FirstAid        | ![FirstAid](img/sample02a.png)        |
| FourthCategory  | ![FourthCategory](img/sample02b.png)  |
| ThirdCategory   | ![ThirdCategory](img/sample02c.png)   |
| SecondCategory  | ![SecondCategory](img/sample03a.png)  |
| FirstCategory   | ![FirstCategory](img/sample03b.png)   |
| HorsCategory    | ![HorsCategory](img/sample03c.png)    |
| Sprint          | ![Sprint](img/sample03d.png)          |
| LeftFork        | (1)                                   |
| RightFork       | (1)                                   |
| MiddleFork      | (1)                                   |
| SlightLeft      | (1)                                   |
| SharpLeft       | (1)                                   |
| SlightRight     | (1)                                   |
| SharpRight      | (1)                                   |
| UTurn           | (1)                                   |
| SegmentStart    | (2)                                   |
| SegmentEnd      | (2)                                   |
| Campsite        | ![Campsite](img/sample06a.png)        |
| AidStation      | ![AidStation](img/sample06b.png)      |
| RestArea        | ![RestArea](img/sample07a.png)        |
| GeneralDistance | ![GeneralDistance](img/sample07b.png) |
| Service         | ![Service](img/sample07c.png)         |
| EnergyGel       | ![EnergyGel](img/sample07d.png)       |
| SportsDrink     | ![SportsDrink](img/sample08a.png)     |
| MileMarker      | ![MileMarker](img/sample08b.png)      |
| Checkpoint      | ![Checkpoint](img/sample08c.png)      |
| Shelter         | ![Shelter](img/sample08d.png)         |
| MeetingSpot     | ![MeetingSpot](img/sample09a.png)     |
| Overlook        | ![Overlook](img/sample09b.png)        |
| Toilet          | ![Toilet](img/sample09c.png)          |
| Shower          | (2)                                   |
| Gear            | ![Gear](img/sample10a.png)            |
| SharpCurve      | ![SharpCurve](img/sample10b.png)      |
| SteepIncline    | ![SteepIncline](img/sample10c.png)    |
| Tunnel          | ![Tunnel](img/sample10d.png)          |
| Bridge          | ![Bridge](img/sample11a.png)          |
| Obstacle        | ![Obstacle](img/sample11b.png)        |
| Crossing        | ![Crossing](img/sample11c.png)        |
| Store           | ![Store](img/sample11d.png)           |
| Transition      | ![Transition](img/sample12a.png)      |
| Navaid          | ![Navaid](img/sample12b.png)          |
| Transport       | ![Transport](img/sample12c.png)       |
| Alert           | ![Alert](img/sample12d.png)           |
| Info            | ![Info](img/sample13a.png)            |

