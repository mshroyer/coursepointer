open FSharp.Data.UnitSystems.SI.UnitSymbols

open Expecto

open CoursePointer.Geodesy

[<Tests>]
let tests =
    test "An example test" {
        let subject = "Hello world"
        Expect.equal "Hello world" subject "The strings should be equal"
        
        let sum = CoursePointer.Say.addSome 2 3
        Expect.equal sum 5 "2+3 should equal 5"
    }
    
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
    runTestsWithCLIArgs [] args tests
