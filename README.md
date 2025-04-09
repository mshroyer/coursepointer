# coursepointer

A Python script for translating Ride with GPS routes, containing Points of Interest, into Course Points for use on
Garmin Edge devices.

## Overview

[Ride with GPS](https://ridewithgps.com/) is a popular tool for planning and sharing cycling routes, and Garmin Edge is
a popular line of bicycle computers. If you're planning or attending a group ride or a tour, it's very likely that the
route will be shared with you on Ride with GPS, and also that many of the participants will be using Garmin Edge devices
to navigate said route. And even on individual rides that I plan and navigate alone, I personally prefer Ride with GPS's
route-planning features to those offered by Garmin's own Connect website and app.

In my experience, one of the most useful features of a Garmin Edge device is its ability to display a dynamic list of
upcoming "Course Points", representing things like water stations, food stops, or towns along the route. Especially on
longer tours, this is a great way to see 

But there's a catch to using a route planned in Ride with GPS on a Garmin Edge: Ride with GPS's "Points of Interest"
—which you might use to annotate things such as water refill stations, rest stops, or towns along your route—do not
translate well to Garmin Edge devices' "Course Points" feature.  In visual terms, I'd like to achieve this:

## Prerequisites

[uv](https://docs.astral.sh/uv/) is a new but increasingly-prevalent tool for developing and running Python projects. In
particular, uv makes it simple to run small, easily-redistributable, single-file Python scripts—such as this one—that
rely on external dependencies (in this case, the [Garmin FIT SDK](https://github.com/garmin/fit-python-sdk)). With uv
you can easily run this script without doing a bunch of complicated development environment setup.
