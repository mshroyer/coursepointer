open FSharp.Data.UnitSystems.SI.UnitSymbols

open Expecto

open CoursePointer.Geodesy
open CoursePointer.Measures

[<Tests>]
let geodesyTests =
    test "Test distance calculation" {
        let p1 = { SurfacePoint.Lat = 0.0<deg>; Lon = 0.0<deg> }
        let p2 = { SurfacePoint.Lat = 5.0<deg>; Lon = 5.0<deg> }
        let distance = getDistance p1 p2
        Expect.isTrue (abs(distance - 784029.0<m>) < 1.0<m>) "Expect distances to be approximately equal"
    }

[<EntryPoint>]
let main args =
    runTestsWithCLIArgs [] args geodesyTests
