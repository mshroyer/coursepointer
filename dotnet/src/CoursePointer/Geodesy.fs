module CoursePointer.Geodesy

open FSharp.Data.UnitSystems.SI.UnitSymbols

[<Measure>] type deg

[<Struct>]
type SurfacePoint =
    { Lat: double<deg>
      Lon: double<deg> }

let getDistance a b : double<m> =
    0.0<m>
