module CoursePointer.Measures

open FSharp.Data.UnitSystems.SI.UnitSymbols

[<Measure>]
type km

[<Measure>]
type ms

[<Measure>]
type hr

(* Unit conversions *)
let mPerKm = 1000.0<m/km>
let sPerHr = 3600.0<s/hr>

[<Measure>]
type deg

[<Measure>]
type semicircle
