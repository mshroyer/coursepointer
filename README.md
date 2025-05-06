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

## Why can't I use Garmin Connect to do this?

Garmin Connect can import GPX tracks with waypoints, and export FIT files containing course points, so naturally many people try to use Garmin's course planner to get their RWGPS POIs onto their devices.  But this won't work directly, and I never understood why until I sat down to study the FIT file format.

The problem is that a course point consists of not only a geolocation, but also a distance along the course.  Edge devices use this distance for the list of upcoming course points.

When you import a GPX track with waypoints into Garmin Connect on the web, those waypoints show up on the map as course points.  By all appearances, you should be all set!  But at the time of writing, Connect doesn't compute the distance field of imported course points, and if you click on the course point to edit it you'll see it's at a distance of zero, despite being in the correct location on the map.

The resulting behavior is that when you export this course as a FIT file and select it for navigation on your device, your course points will all show up on the map and the course point screen—right until you start the course and pass distance zero, after which they'll all disappear from the course point list.

Needless to say, this behavior is confusing, and for a long time I chalked it off as a bug in my Edge device.

As far as I can tell, the only way to work around this in the Garmin Connect course planner is to delete your imported course points and recreate them, one by one.

## Why not use custom cues?

## Dist. next etc.
