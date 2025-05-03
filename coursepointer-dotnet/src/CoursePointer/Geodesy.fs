module CoursePointer.Geodesy

open FSharp.Data.UnitSystems.SI.UnitSymbols

open GeographicLib

[<Measure>]
type deg

[<Measure>]
type semicircle

[<Struct>]
type SurfacePoint = { Lat: float<deg>; Lon: float<deg> }

[<Struct>]
type InverseResult = { Length: float<m>; Azimuth1: float<deg>; Azimuth2: float<deg> }

let semicircles (degrees: float<deg>): int<semicircle> =
    let rawDegrees = float degrees
    int (System.Math.Round(rawDegrees * 2.0**31 / 180.0)) * 1<semicircle>

let getDistance a b : float<m> =
    let result =
        Geodesic.WGS84.Inverse(
            double a.Lat,
            double a.Lon,
            double b.Lat,
            double b.Lon,
            GeodesicFlags.Distance ||| GeodesicFlags.Azimuth
        )
    result.Distance * 1.0<m>
