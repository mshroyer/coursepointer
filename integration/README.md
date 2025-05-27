# Integration tests

This directory contains Python-based integration tests of the Rust CLI.

## Data files

- cptr001.fit: A simple course created on Garmin Connect on the web, with no course points.
- cptr002.{gpx,fit}: A freehand course created on Ride with GPS with a start, midpoint, and end along the bay trail, of about 1.06km in length, exported as both a GPX track and FIT.
- cptr003.{gpx,fit}: RWGPS route up Old La Honda and along Skyline totalling 30 miles, with four POIs along the way, as both GPX track with POIs as waypoints and FIT.
- cptr003_connect.fit: cptr003.gpx imported into Garmin Connect and re-exported as FIT.
- cptr004.gpx: Export of [a Gaia GPS folder](https://www.gaiagps.com/map/?loc=13.6/-122.1094/37.2305&pubLink=b4sitlz7aVCkPFVDFaLm1bSi&folderId=16650ba9-3ecb-45cb-9c21-430e53c7d4c0) folder containing a hiking route and five waypoints, four of them located approximately on the route.
