module CoursePointer.Geodesy

open FSharp.Data.UnitSystems.SI.UnitSymbols

open GeographicLib

[<Measure>]
type deg

[<Struct>]
type SurfacePoint = { Lat: double<deg>; Lon: double<deg> }

[<Struct>]
type InverseResult = { Length: double<m>; Azimuth1: double<deg>; Azimuth2: double<deg> }

let getDistance a b : double<m> =
    let result =
        Geodesic.WGS84.Inverse(
            double a.Lat,
            double a.Lon,
            double b.Lat,
            double b.Lon,
            GeodesicFlags.Distance ||| GeodesicFlags.Azimuth
        )
    result.Distance * 1.0<m>
