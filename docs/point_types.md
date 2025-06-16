# Course Point Types

The canonical set of course point types is defined in `Profile.xlsx` in the
[Garmin FIT SDK](https://developer.garmin.com/fit/download/), as the
`course_point` enum in the Types tab.  But the real question is how these
behave in practice on devices.

This document's purpose is to identify:

1. What subset of course point types is safe to use across devices.
2. How to map exported waypoints from popular apps like Ride with GPS and Gaia
   GPS to that subset.

## On Garmin apps and devices

Here's how different course points function in Garmin Connect and appear in
"Up Ahead" on a Fenix 7 with software version 21.19, as well as an Edge 1040
with software 27.14.

There are a few different cases to consider: Types listing an icon appear in
Up Ahead with that icon, and also on the course map with a similar but
monochrome icon.  Types marked with (1) are missing entirely from Up Ahead,
but while they lack an icon on the map, the course point's name will appear in
the correct map location.  Finally, types marked with (2) are missing from Up
Ahead but appear on the map with an appropriate icon.

| Type               | Connect | Fenix 7                               | Edge 1040                             |
|--------------------|---------|---------------------------------------|---------------------------------------|
| `generic`          | yes     | ![Generic](img/sample00a.png)         | ![Generic](img/edge1040sample00a.png) |
| `summit`           | yes     | ![Summit](img/sample00b.png)          | ![Generic](img/edge1040sample00b.png) |
| `valley`           | yes     | ![Valley](img/sample00c.png)          | ![Generic](img/edge1040sample00c.png) |
| `water`            | yes     | ![Water](img/sample00d.png)           | ![Generic](img/edge1040sample00d.png) |
| `food`             | yes     | ![Food](img/sample01a.png)            | ![Generic](img/edge1040sample01a.png) |
| `danger`           | yes     | ![Danger](img/sample01b.png)          | ![Generic](img/edge1040sample01b.png) |
| `left`             | no      | (1)                                   | ![Generic](img/edge1040sample01c.png) |
| `right`            | no      | (1)                                   | ![Generic](img/edge1040sample01d.png) |
| `straight`         | no      | (1)                                   | ![Generic](img/edge1040sample02a.png) |
| `first_aid`        | yes     | ![FirstAid](img/sample02a.png)        | ![Generic](img/edge1040sample02b.png) |
| `fourth_category`  | yes     | ![FourthCategory](img/sample02b.png)  | ![Generic](img/edge1040sample02c.png) |
| `third_category`   | yes     | ![ThirdCategory](img/sample02c.png)   | ![Generic](img/edge1040sample02d.png) |
| `second_category`  | yes     | ![SecondCategory](img/sample03a.png)  | ![Generic](img/edge1040sample03a.png) |
| `first_category`   | yes     | ![FirstCategory](img/sample03b.png)   | ![Generic](img/edge1040sample03b.png) |
| `hors_category`    | yes     | ![HorsCategory](img/sample03c.png)    | ![Generic](img/edge1040sample03c.png) |
| `sprint`           | yes     | ![Sprint](img/sample03d.png)          | ![Generic](img/edge1040sample03d.png) |
| `left_fork`        | no      | (1)                                   | ![Generic](img/edge1040sample04a.png) |
| `right_fork`       | no      | (1)                                   | ![Generic](img/edge1040sample04b.png) |
| `middle_fork`      | no      | (1)                                   | ![Generic](img/edge1040sample04c.png) |
| `slight_left`      | no      | (1)                                   | ![Generic](img/edge1040sample04d.png) |
| `sharp_left`       | no      | (1)                                   | ![Generic](img/edge1040sample05a.png) |
| `slight_right`     | no      | (1)                                   | ![Generic](img/edge1040sample05b.png) |
| `sharp_right`      | no      | (1)                                   | ![Generic](img/edge1040sample05c.png) |
| `u_turn`           | no      | (1)                                   | ![Generic](img/edge1040sample05d.png) |
| `segment_start`    | no      | (2)                                   | (1)                                   |
| `segment_end`      | no      | (2)                                   | ![Generic](img/edge1040sample06a.png) |
| `campsite`         | yes     | ![Campsite](img/sample06a.png)        | ![Generic](img/edge1040sample06b.png) |
| `aid_station`      | yes     | ![AidStation](img/sample06b.png)      | ![Generic](img/edge1040sample06c.png) |
| `rest_area`        | yes     | ![RestArea](img/sample07a.png)        | ![Generic](img/edge1040sample07a.png) |
| `general_distance` | yes     | ![GeneralDistance](img/sample07b.png) | ![Generic](img/edge1040sample07b.png) |
| `service`          | yes     | ![Service](img/sample07c.png)         | ![Generic](img/edge1040sample07c.png) |
| `energy_gel`       | yes     | ![EnergyGel](img/sample07d.png)       | ![Generic](img/edge1040sample07d.png) |
| `sports_drink`     | yes     | ![SportsDrink](img/sample08a.png)     | ![Generic](img/edge1040sample08a.png) |
| `mile_marker`      | yes     | ![MileMarker](img/sample08b.png)      | ![Generic](img/edge1040sample08b.png) |
| `checkpoint`       | yes     | ![Checkpoint](img/sample08c.png)      | ![Generic](img/edge1040sample08c.png) |
| `shelter`          | yes     | ![Shelter](img/sample08d.png)         | ![Generic](img/edge1040sample08d.png) |
| `meeting_spot`     | yes     | ![MeetingSpot](img/sample09a.png)     | ![Generic](img/edge1040sample09a.png) |
| `overlook`         | yes     | ![Overlook](img/sample09b.png)        | ![Generic](img/edge1040sample09b.png) |
| `toilet`           | yes     | ![Toilet](img/sample09c.png)          | ![Generic](img/edge1040sample09c.png) |
| `shower`           | yes     | ![Shower](img/sample09d.png)          | ![Generic](img/edge1040sample09d.png) |
| `gear`             | yes     | ![Gear](img/sample10a.png)            | ![Generic](img/edge1040sample10a.png) |
| `sharp_curve`      | yes     | ![SharpCurve](img/sample10b.png)      | ![Generic](img/edge1040sample10b.png) |
| `steep_incline`    | yes     | ![SteepIncline](img/sample10c.png)    | ![Generic](img/edge1040sample10c.png) |
| `tunnel`           | yes     | ![Tunnel](img/sample10d.png)          | ![Generic](img/edge1040sample10d.png) |
| `bridge`           | yes     | ![Bridge](img/sample11a.png)          | ![Generic](img/edge1040sample11a.png) |
| `obstacle`         | yes     | ![Obstacle](img/sample11b.png)        | ![Generic](img/edge1040sample11b.png) |
| `crossing`         | yes     | ![Crossing](img/sample11c.png)        | ![Generic](img/edge1040sample11c.png) |
| `store`            | yes     | ![Store](img/sample11d.png)           | ![Generic](img/edge1040sample11d.png) |
| `transition`       | yes     | ![Transition](img/sample12a.png)      | ![Generic](img/edge1040sample12a.png) |
| `navaid`           | yes     | ![Navaid](img/sample12b.png)          | ![Generic](img/edge1040sample12b.png) |
| `transport`        | yes     | ![Transport](img/sample12c.png)       | ![Generic](img/edge1040sample12c.png) |
| `alert`            | yes     | ![Alert](img/sample12d.png)           | ![Generic](img/edge1040sample12d.png) |
| `info`             | yes     | ![Info](img/sample13a.png)            | ![Generic](img/edge1040sample13a.png) |

Bizarrely, the `shower` course point didn't show up at all my first time
testing this on my Fenix, but then rendered the next time, with the exact same
course file and firmware version.

The Connect column indicates whether the course point type appears when
imported into Garmin Connect, or can be created manually.  As of 2025-06-15,
it's possible to create additional "Obstacle Start" (type number 54) and
"Obstacle End" (type 55) which are absent from the current global
`Profile.xlsx`.

I did encounter some unexpected behavior when importing synthetically
generated courses (which did not correspond to any real trail or road) into
Connect.  Particularly, importing such FIT files containing more than four
course points resulted in *no* course points appearing.  I haven't yet
reproduced this behavior with conversions of "real" courses, however.

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

All POI types set `Dot` as `sym` in the GPX export.

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

## Gaia GPS Waypoints

Gaia GPS supports an _enormous_ number of waypoint types, including various
emoji.  But a subset of these, the "Gaia" set, bear semantics that map well
onto the (smaller) set of course point types.

In GPX exports, Gaia GPS indicates the point type by the text in the
waypoint's `sym` sub-element.  These basically, but not precisely, match the
waypoint's type; many have a `-24` suffix, which presumably corresponds to an
icon size.  With others, such as `:gaia-body-of-water:`, the `sym` text is
simplified to `water` for example.

| Type                | Icon                                    | Sym                      |
|---------------------|-----------------------------------------|--------------------------|
| `:gaia-pin:`                 | ![gaia-icon-0](img/gaia-icon-0.png)     | None                     |
| `:gaia-airport:`             | ![gaia-icon-1](img/gaia-icon-1.png)     | `airport-24`             |
| `:gaia-attraction:`          | ![gaia-icon-2](img/gaia-icon-2.png)     | `attraction`             |
| `:gaia-beach:`               | ![gaia-icon-3](img/gaia-icon-3.png)     | `beach`                  |
| `:gaia-bear:`                | ![gaia-icon-4](img/gaia-icon-4.png)     | `bear`                   |
| `:gaia-bicycle:`             | ![gaia-icon-5](img/gaia-icon-5.png)     | `bicycle-24`             |
| `:gaia-binoculars:`          | ![gaia-icon-6](img/gaia-icon-6.png)     | `binoculars`             |
| `:gaia-bird:`                | ![gaia-icon-7](img/gaia-icon-7.png)     | `bird`                   |
| `:gaia-body-of-water:`       | ![gaia-icon-8](img/gaia-icon-8.png)     | `water`                  |
| `:gaia-bridge:`              | ![gaia-icon-9](img/gaia-icon-9.png)     | `bridge`                 |
| `:gaia-building:`            | ![gaia-icon-10](img/gaia-icon-10.png)   | `building-24`            |
| `:gaia-bus:`                 | ![gaia-icon-11](img/gaia-icon-11.png)   | `bus`                    |
| `:gaia-cafe:`                | ![gaia-icon-12](img/gaia-icon-12.png)   | `cafe-24`                |
| `:gaia-cairn:`               | ![gaia-icon-13](img/gaia-icon-13.png)   | `cairn`                  |
| `:gaia-camera:`              | ![gaia-icon-14](img/gaia-icon-14.png)   | `camera-24`              |
| `:gaia-campsite:`            | ![gaia-icon-15](img/gaia-icon-15.png)   | `campsite-24`            |
| `:gaia-canoe:`               | ![gaia-icon-16](img/gaia-icon-16.png)   | `canoe`                  |
| `:gaia-car:`                 | ![gaia-icon-17](img/gaia-icon-17.png)   | `car-24`                 |
| `:gaia-cave:`                | ![gaia-icon-18](img/gaia-icon-18.png)   | `cave`                   |
| `:gaia-cemetery:`            | ![gaia-icon-19](img/gaia-icon-19.png)   | `cemetery-24`            |
| `:gaia-chemist:`             | ![gaia-icon-20](img/gaia-icon-20.png)   | `chemist-24`             |
| `:gaia-circle:`              | ![gaia-icon-21](img/gaia-icon-21.png)   | `circle-24`              |
| `:gaia-city:`                | ![gaia-icon-22](img/gaia-icon-22.png)   | `city-24`                |
| `:gaia-cliff:`               | ![gaia-icon-23](img/gaia-icon-23.png)   | `cliff`                  |
| `:gaia-climbing:`            | ![gaia-icon-24](img/gaia-icon-24.png)   | `climbing`               |
| `:gaia-dam:`                 | ![gaia-icon-25](img/gaia-icon-25.png)   | `dam-24`                 |
| `:gaia-danger:`              | ![gaia-icon-26](img/gaia-icon-26.png)   | `danger-24`              |
| `:gaia-deer:`                | ![gaia-icon-27](img/gaia-icon-27.png)   | `deer`                   |
| `:gaia-disability:`          | ![gaia-icon-28](img/gaia-icon-28.png)   | `disability-24`          |
| `:gaia-disc-golf:`           | ![gaia-icon-29](img/gaia-icon-29.png)   | `disc-golf`              |
| `:gaia-dog-park:`            | ![gaia-icon-30](img/gaia-icon-30.png)   | `dog-park-24`            |
| `:gaia-electric:`            | ![gaia-icon-31](img/gaia-icon-31.png)   | `electric`               |
| `:gaia-elk:`                 | ![gaia-icon-32](img/gaia-icon-32.png)   | `elk`                    |
| `:gaia-emergency-telephone:` | ![gaia-icon-33](img/gaia-icon-33.png)   | `emergency-telephone-24` |
| `:gaia-fast-food:`           | ![gaia-icon-34](img/gaia-icon-34.png)   | `fast-food-24`           |
| `:gaia-fence:`               | ![gaia-icon-35](img/gaia-icon-35.png)   | `fence`                  |
| `:gaia-fire-lookout:`        | ![gaia-icon-36](img/gaia-icon-36.png)   | `fire-lookout`           |
| `:gaia-fire-station:`        | ![gaia-icon-37](img/gaia-icon-37.png)   | `fire-station-24`        |
| `:gaia-fish:`                | ![gaia-icon-38](img/gaia-icon-38.png)   | `fish`                   |
| `:gaia-forest:`              | ![gaia-icon-39](img/gaia-icon-39.png)   | `forest`                 |
| `:gaia-fuel:`                | ![gaia-icon-40](img/gaia-icon-40.png)   | `fuel-24`                |
| `:gaia-garden:`              | ![gaia-icon-41](img/gaia-icon-41.png)   | `garden-24`              |
| `:gaia-gate:`                | ![gaia-icon-42](img/gaia-icon-42.png)   | `gate`                   |
| `:gaia-geyser:`              | ![gaia-icon-43](img/gaia-icon-43.png)   | `geyser`                 |
| `:gaia-ghost-town:`          | ![gaia-icon-44](img/gaia-icon-44.png)   | `ghost-town`             |
| `:gaia-golf:`                | ![gaia-icon-45](img/gaia-icon-45.png)   | `golf-24`                |
| `:gaia-grass:`               | ![gaia-icon-46](img/gaia-icon-46.png)   | `grass`                  |
| `:gaia-ground-blind:`        | ![gaia-icon-47](img/gaia-icon-47.png)   | `ground-blind`           |
| `:gaia-harbor:`              | ![gaia-icon-48](img/gaia-icon-48.png)   | `harbor-24`              |
| `:gaia-heart:`               | ![gaia-icon-49](img/gaia-icon-49.png)   | `heart-24`               |
| `:gaia-helipad:`             | ![gaia-icon-50](img/gaia-icon-50.png)   | `helipad`                |
| `:gaia-heliport:`            | ![gaia-icon-51](img/gaia-icon-51.png)   | `heliport-24`            |
| `:gaia-horse:`               | ![gaia-icon-52](img/gaia-icon-52.png)   | `horse`                  |
| `:gaia-hospital:`            | ![gaia-icon-53](img/gaia-icon-53.png)   | `hospital-24`            |
| `:gaia-hotspring:`           | ![gaia-icon-54](img/gaia-icon-54.png)   | `hotspring`              |
| `:gaia-information:`         | ![gaia-icon-55](img/gaia-icon-55.png)   | `information`            |
| `:gaia-known-route:`         | ![gaia-icon-56](img/gaia-icon-56.png)   | `known-route`            |
| `:gaia-lake:`                | ![gaia-icon-57](img/gaia-icon-57.png)   | `lake`                   |
| `:gaia-lighthouse:`          | ![gaia-icon-58](img/gaia-icon-58.png)   | `lighthouse-24`          |
| `:gaia-lodging:`             | ![gaia-icon-59](img/gaia-icon-59.png)   | `lodging-24`             |
| `:gaia-logging:`             | ![gaia-icon-60](img/gaia-icon-60.png)   | `logging-24`             |
| `:gaia-market:`              | ![gaia-icon-61](img/gaia-icon-61.png)   | `market`                 |
| `:gaia-marsh:`               | ![gaia-icon-62](img/gaia-icon-62.png)   | `marsh`                  |
| `:gaia-mine:`                | ![gaia-icon-63](img/gaia-icon-63.png)   | `mine`                   |
| `:gaia-minefield:`           | ![gaia-icon-64](img/gaia-icon-64.png)   | `minefield-24`           |
| `:gaia-mobilephone:`         | ![gaia-icon-65](img/gaia-icon-65.png)   | `mobilephone-24`         |
| `:gaia-moose:`               | ![gaia-icon-66](img/gaia-icon-66.png)   | `moose`                  |
| `:gaia-mud:`                 | ![gaia-icon-67](img/gaia-icon-67.png)   | `mud`                    |
| `:gaia-museum:`              | ![gaia-icon-68](img/gaia-icon-68.png)   | `museum`                 |
| `:gaia-mushroom:`            | ![gaia-icon-69](img/gaia-icon-69.png)   | `mushroom`               |
| `:gaia-natural-spring:`      | ![gaia-icon-70](img/gaia-icon-70.png)   | `natural-spring`         |
| `:gaia-no-admittance-1:`     | ![gaia-icon-71](img/gaia-icon-71.png)   | `no-admittance-1`        |
| `:gaia-no-admittance-2:`     | ![gaia-icon-72](img/gaia-icon-72.png)   | `no-admittance-2`        |
| `:gaia-number-0:`            | ![gaia-icon-73](img/gaia-icon-73.png)   | `number-0`               |
| `:gaia-number-1:`            | ![gaia-icon-74](img/gaia-icon-74.png)   | `number-1`               |
| `:gaia-number-2:`            | ![gaia-icon-75](img/gaia-icon-75.png)   | `number-2`               |
| `:gaia-number-3:`            | ![gaia-icon-76](img/gaia-icon-76.png)   | `number-3`               |
| `:gaia-number-4:`            | ![gaia-icon-77](img/gaia-icon-77.png)   | `number-4`               |
| `:gaia-number-5:`            | ![gaia-icon-78](img/gaia-icon-78.png)   | `number-5`               |
| `:gaia-number-6:`            | ![gaia-icon-79](img/gaia-icon-79.png)   | `number-6`               |
| `:gaia-number-7:`            | ![gaia-icon-80](img/gaia-icon-80.png)   | `number-7`               |
| `:gaia-number-8:`            | ![gaia-icon-81](img/gaia-icon-81.png)   | `number-8`               |
| `:gaia-number-9:`            | ![gaia-icon-82](img/gaia-icon-82.png)   | `number-9`               |
| `:gaia-off-road:`            | ![gaia-icon-83](img/gaia-icon-83.png)   | `off-road`               |
| `:gaia-oil-well:`            | ![gaia-icon-84](img/gaia-icon-84.png)   | `oil-well-24`            |
| `:gaia-park:`                | ![gaia-icon-85](img/gaia-icon-85.png)   | `park-24`                |
| `:gaia-parking:`             | ![gaia-icon-86](img/gaia-icon-86.png)   | `parking-24`             |
| `:gaia-peak:`                | ![gaia-icon-87](img/gaia-icon-87.png)   | `peak`                   |
| `:gaia-petroglyph:`          | ![gaia-icon-88](img/gaia-icon-88.png)   | `petroglyph`             |
| `:gaia-picnic:`              | ![gaia-icon-89](img/gaia-icon-89.png)   | `picnic`                 |
| `:gaia-pitch:`               | ![gaia-icon-90](img/gaia-icon-90.png)   | `pitch-24`               |
| `:gaia-playground:`          | ![gaia-icon-91](img/gaia-icon-91.png)   | `playground-24`          |
| `:gaia-police:`              | ![gaia-icon-92](img/gaia-icon-92.png)   | `police`                 |
| `:gaia-polling-place:`       | ![gaia-icon-93](img/gaia-icon-93.png)   | `polling-place-24`       |
| `:gaia-potable-water:`       | ![gaia-icon-94](img/gaia-icon-94.png)   | `potable-water`          |
| `:gaia-prison:`              | ![gaia-icon-95](img/gaia-icon-95.png)   | `prison-24`              |
| `:gaia-rail:`                | ![gaia-icon-96](img/gaia-icon-96.png)   | `rail-24`                |
| `:gaia-railroad:`            | ![gaia-icon-97](img/gaia-icon-97.png)   | `railroad`               |
| `:gaia-ranger-station:`      | ![gaia-icon-98](img/gaia-icon-98.png)   | `ranger-station`         |
| `:gaia-reef:`                | ![gaia-icon-99](img/gaia-icon-99.png)   | `reef`                   |
| `:gaia-restaurant:`          | ![gaia-icon-100](img/gaia-icon-100.png) | `restaurant-24`          |
| `:gaia-resupply:`            | ![gaia-icon-101](img/gaia-icon-101.png) | `resupply`               |
| `:gaia-ruins:`               | ![gaia-icon-102](img/gaia-icon-102.png) | `ruins`                  |
| `:gaia-rv-park:`             | ![gaia-icon-103](img/gaia-icon-103.png) | `rv-park`                |
| `:gaia-saddle:`              | ![gaia-icon-104](img/gaia-icon-104.png) | `saddle`                 |
| `:gaia-sand-dune:`           | ![gaia-icon-105](img/gaia-icon-105.png) | `sand-dune`              |
| `:gaia-scat:`                | ![gaia-icon-106](img/gaia-icon-106.png) | `scat`                   |
| `:gaia-scrub:`               | ![gaia-icon-107](img/gaia-icon-107.png) | `scrub`                  |
| `:gaia-shelter:`             | ![gaia-icon-108](img/gaia-icon-108.png) | `shelter`                |
| `:gaia-shower:`              | ![gaia-icon-109](img/gaia-icon-109.png) | `shower`                 |
| `:gaia-skiing:`              | ![gaia-icon-110](img/gaia-icon-110.png) | `skiing-24`              |
| `:gaia-snowflake:`           | ![gaia-icon-111](img/gaia-icon-111.png) | `snowflake`              |
| `:gaia-snowmobile:`          | ![gaia-icon-112](img/gaia-icon-112.png) | `snowmobile`             |
| `:gaia-square:`              | ![gaia-icon-113](img/gaia-icon-113.png) | `square-24`              |
| `:gaia-star:`                | ![gaia-icon-114](img/gaia-icon-114.png) | `star-24`                |
| `:gaia-steps:`               | ![gaia-icon-115](img/gaia-icon-115.png) | `steps`                  |
| `:gaia-stone:`               | ![gaia-icon-116](img/gaia-icon-116.png) | `stone`                  |
| `:gaia-suitcase:`            | ![gaia-icon-117](img/gaia-icon-117.png) | `suitcase-24`            |
| `:gaia-swimming:`            | ![gaia-icon-118](img/gaia-icon-118.png) | `swimming-24`            |
| `:gaia-toilets:`             | ![gaia-icon-119](img/gaia-icon-119.png) | `toilets-24`             |
| `:gaia-track-deer:`          | ![gaia-icon-120](img/gaia-icon-120.png) | `track-deer`             |
| `:gaia-track-dog:`           | ![gaia-icon-121](img/gaia-icon-121.png) | `track-dog`              |
| `:gaia-trail-camera:`        | ![gaia-icon-122](img/gaia-icon-122.png) | `trail-camera`           |
| `:gaia-trailhead:`           | ![gaia-icon-123](img/gaia-icon-123.png) | `trailhead`              |
| `:gaia-trash:`               | ![gaia-icon-124](img/gaia-icon-124.png) | `trash`                  |
| `:gaia-tree-fall:`           | ![gaia-icon-125](img/gaia-icon-125.png) | `tree-fall`              |
| `:gaia-tree-stand:`          | ![gaia-icon-126](img/gaia-icon-126.png) | `tree-stand`             |
| `:gaia-triangle:`            | ![gaia-icon-127](img/gaia-icon-127.png) | `triangle-24`            |
| `:gaia-turkey:`              | ![gaia-icon-128](img/gaia-icon-128.png) | `turkey`                 |
| `:gaia-volcano:`             | ![gaia-icon-129](img/gaia-icon-129.png) | `volcano`                |
| `:gaia-water:`               | ![gaia-icon-130](img/gaia-icon-130.png) | `water-24`               |
| `:gaia-waterfall:`           | ![gaia-icon-131](img/gaia-icon-131.png) | `waterfall`              |
| `:gaia-wetland:`             | ![gaia-icon-132](img/gaia-icon-132.png) | `wetland-24`             |
| `:gaia-wood:`                | ![gaia-icon-133](img/gaia-icon-133.png) | `wood`                   |
