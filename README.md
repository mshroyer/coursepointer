# coursepointer

This is a small tool that converts waypoints in a GPX track file into "course points" for use on Garmin devices.  I made this so that when using a cycling course authored in [Ride With GPS](https://ridewithgps.com/), its points of interest (POIs) will appear as course points on my Garmin Edge computer.

# FAQ

## Why is this tool even needed?

First, we have to understand that a modern Garmin Edge device, like the 1040, has two different representations of points of interest along a course.  Course points show up on the map when navigating a course and, most helpfully, in a dedicated data screen that lists them in order with the remaining distance and predicted time to each:

![A list of course points](docs/img/course-point-list.png)

![Course points on the map](docs/img/course-point-map.png)

Saved locations, meanwhile, are saved "globally" on the device—that is to say, not within the context of navigating a specific course.  These will appear on the map, but not in the list of upcoming course points.  Unlike course points, you can ask the device to navigate you to these.

RWGPS can export your route's POIs as waypoints in a GPX track file, but when this file is imported on a Garmin device the waypoints become saved locations.  So they'll show up on the map, but not as upcoming course points.

Unlike GPX, the newer FIT (and also TCX) file format can specify course points, so we can solve this by going through a tool that can convert a GPX track with waypoints into a FIT course with course points.  This tool provides a convenient way to do that.

## Why don't POIs already work when re-exported from Garmin Connect?

## Why not use custom cues?

## Dist. next etc.
