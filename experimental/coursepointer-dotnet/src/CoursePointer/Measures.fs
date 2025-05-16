module CoursePointer.Measures

open FSharp.Data.UnitSystems.SI.UnitSymbols

/// Kilometer
[<Measure>]
type km

/// Millisecond
[<Measure>]
type ms

/// Hour
[<Measure>]
type h

/// Degree
[<Measure>]
type deg

/// Semicircle
[<Measure>]
type semicircle

// Unit conversions
let mPerKm = 1000.0<m/km>
let sPerH = 3600.0<s/h>
