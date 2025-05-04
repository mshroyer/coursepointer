open System

open CoursePointer
open CoursePointer.Geodesy

[<EntryPoint>]
let main _ =
    printfn "Writing test course"
    using (new CourseWriter("out.fit", "Test Course", DateTime.Now, 20.0<km/hr>)) <| fun writer ->
        writer.AddRecord({ Lat = 52.0<deg>; Lon = 13.0<deg> })
        writer.AddRecord({ Lat = 52.1<deg>; Lon = 13.1<deg> })
        writer.AddRecord({ Lat = 52.2<deg>; Lon = 13.2<deg> })
    0
