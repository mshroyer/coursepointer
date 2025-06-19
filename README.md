# CoursePointer

A command-line tool that converts GPX routes/tracks and waypoints into Garmin
FIT course files with "course points". This allows your waypoints to appear in
[Up Ahead](https://support.garmin.com/en-US/?faq=lQMibRoY2I5Y4pP8EXgxv7) on
recent Garmin watches and bicycle computers, displaying their 

## Usage example

When planning a hiking route on [Gaia GPS](https://gaiagps.com/), you can put
a route and waypoints together in a Saved Items folder and then export the
entire folder as a GPX file:

![Example hike](docs/img/gaia-rancho-wildcat.png)

Run coursepointer on the GPX file to produce a FIT file:

```
% coursepointer convert-gpx rancho-wildcat.gpx rancho-wildcat.fit
Converted course "Rancho Wildcat" of length 3.03 mi

Processed 5 waypoints, 5 of which were identified as course points:
- Wildcat loop at 1.09 mi along the course
- Vista point at 1.68 mi
- Toilets at 2.70 mi
- Drinking water at 2.74 mi
- Deer Hollow Farm at 2.82 mi

Output is in /Users/mshroyer/Desktop/rancho-wildcat.fit
```

Copy the FIT file to the device over USB, or import it into [Garmin
Connect](https://connect.garmin.com/modern/) and then send it to your device
from the Garmin mobile app.  Then, when navigating the course your course
points will appear in Up Ahead on compatible devices:

![Garmin Fenix Up Ahead screenshot](docs/img/gaia-rancho-wildcat-screenshot.png)

## Development

See [docs/development.md](docs/development.md).
